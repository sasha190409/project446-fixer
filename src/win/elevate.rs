//! Elevation check and re-launch via ShellExecuteExW("runas").
//!
//! Зачем нужен, если в манифесте уже `requireAdministrator`:
//! манифест уважается только при обычном запуске. `__COMPAT_LAYER=RunAsInvoker`,
//! Task Scheduler с "run with highest privileges = off", GPO с тихим отказом,
//! запуск из не-elevated IDE — во всех этих случаях процесс стартует
//! не-elevated, и `netsh`/`hosts`/`TerminateProcess` молча падают по правам.
//! `ensure_elevated()` перезапускает через `runas` → UAC → работаем.

use anyhow::{Context, Result};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Security::{
    GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
};
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetExitCodeProcess, OpenProcessToken, WaitForSingleObject, INFINITE,
};
use windows::Win32::UI::Shell::{
    ShellExecuteExW, SHELLEXECUTEINFOW, SEE_MASK_NOCLOSEPROCESS,
};
use windows::Win32::UI::WindowsAndMessaging::{
    MessageBoxW, MB_ICONINFORMATION, MB_ICONERROR, MB_OK,
};

/// Скрытый CLI-флаг, который мы добавляем в аргументы при re-launch.
/// Ребёнок видит его и не пытается перезапуститься снова — иначе
/// получим бесконечный цикл на системах, где UAC тихо отказывает.
const ELEVATION_MARKER: &str = "--elevation-attempted";

fn to_wide(s: &OsStr) -> Vec<u16> {
    s.encode_wide().chain(std::iter::once(0)).collect()
}

fn wide_str(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn is_elevated() -> bool {
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION::default();
        let mut ret_len = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut ret_len,
        )
        .is_ok();
        let _ = CloseHandle(token);
        ok && elevation.TokenIsElevated != 0
    }
}

fn show_message(title: &str, text: &str, error: bool) {
    let tw = wide_str(title);
    let pw = wide_str(text);
    let style = if error { MB_ICONERROR } else { MB_ICONINFORMATION } | MB_OK;
    unsafe {
        let _ = MessageBoxW(None, PCWSTR(pw.as_ptr()), PCWSTR(tw.as_ptr()), style);
    }
}

fn relaunch_elevated() -> Result<()> {
    let exe = std::env::current_exe().context("current_exe")?;

    // Аргументы + маркер, чтобы ребёнок знал, что elevation уже пробовали.
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if !args.iter().any(|a| a == ELEVATION_MARKER) {
        args.push(ELEVATION_MARKER.to_string());
    }
    let args_joined = args
        .iter()
        .map(|a| if a.contains(' ') { format!("\"{}\"", a) } else { a.clone() })
        .collect::<Vec<_>>()
        .join(" ");

    let exe_w = to_wide(exe.as_os_str());
    let args_w = to_wide(OsStr::new(&args_joined));
    let verb_w = to_wide(OsStr::new("runas"));

    unsafe {
        let mut info = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS,
            lpVerb: PCWSTR(verb_w.as_ptr()),
            lpFile: PCWSTR(exe_w.as_ptr()),
            lpParameters: PCWSTR(args_w.as_ptr()),
            nShow: 1,
            ..Default::default()
        };

        // ShellExecuteExW возвращает Err при отказе UAC (обычно
        // ERROR_CANCELLED = 1223) и при политике GPO (ERROR_ACCESS_DENIED).
        // Оба варианта — fail-soft, покажем сообщение и выйдем.
        if let Err(e) = ShellExecuteExW(&mut info) {
            anyhow::bail!("elevation declined or blocked: {e}");
        }

        // Некоторые драйверы UAC возвращают Ok с нулевым hProcess
        // при нажатии «Нет». Ловим и это.
        if info.hProcess.is_invalid() {
            anyhow::bail!("elevation cancelled by user");
        }

        // Ждём ребёнка, чтобы путь завершения был единым:
        // parent exit code == child exit code, пользователь видит
        // одну «сессию», а не два независимых процесса.
        WaitForSingleObject(info.hProcess, INFINITE);
        let mut code = 0u32;
        let _ = GetExitCodeProcess(info.hProcess, &mut code);
        let _ = CloseHandle(info.hProcess);
        std::process::exit(code as i32);
    }
}

pub fn ensure_elevated() -> Result<()> {
    if is_elevated() {
        return Ok(());
    }

    // Рекурсия: если родитель перезапустил нас, а токен всё ещё не elevated,
    // значит UAC отключён GPO или запуск через RunAsInvoker не сработал.
    // Второй раз не пытаемся — иначе цикл.
    if std::env::args().any(|a| a == ELEVATION_MARKER) {
        anyhow::bail!(
            "administrator privileges are required, but elevation did not take effect.\n\
             UAC may be disabled by group policy, or the process was started with RunAsInvoker.\n\n\
             Требуются права администратора, но повышение не сработало.\n\
             Возможно, UAC отключён групповой политикой или процесс запущен через RunAsInvoker."
        );
    }

    // Двуязычно: язык из реестра ещё не прочитан (это делается в gui::App::new).
    show_message(
        "CS:GO Legacy Fixer",
        "Administrator privileges are required.\n\
         The application will now re-launch elevated.\n\n\
         Требуются права администратора.\n\
         Приложение сейчас перезапустится с повышением.",
        false,
    );

    if let Err(e) = relaunch_elevated() {
        show_message(
            "CS:GO Legacy Fixer",
            &format!(
                "Could not elevate the process.\n\
                 Не удалось повысить права.\n\n{e:#}"
            ),
            true,
        );
        return Err(e);
    }

    // Сюда попадаем только если relaunch_elevated вернул Ok без запуска
    // дочернего процесса (не должно случаться, но на всякий случай).
    std::process::exit(0);
}
