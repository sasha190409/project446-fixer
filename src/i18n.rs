//! Localisation and logging bootstrap.

use anyhow::{Context, Result};
use crate::args::Lang;
use std::path::PathBuf;

pub struct Messages {
    pub title: &'static str,

    pub killing: &'static str,
    pub killing_csgo: &'static str,

    pub fix_update: &'static str,
    pub downloading: &'static str,
    pub download_fail: &'static str,
    pub hash_ok: &'static str,
    pub hash_fail: &'static str,
    pub manifest_fail: &'static str,
    pub manifest_incomplete: &'static str,
    pub sig_missing: &'static str,
    pub sig_created: &'static str,
    pub copy_ok: &'static str,
    pub copy_fail: &'static str,
    pub update_done: &'static str,
    pub disk_low: &'static str,
    pub version: &'static str,
    pub size: &'static str,

    // ---- Signature verification ----
    pub sig_verified: &'static str,
    pub sig_skip_no_key: &'static str,
    pub sig_fail: &'static str,
    pub sig_bad_key: &'static str,

    // ---- Cancel ----
    pub cancel: &'static str,
    pub cancelling: &'static str,
    pub cancelled: &'static str,

    // ---- Validate fix (kicked from server) ----
    pub fix_validate: &'static str,
    pub validate_wait: &'static str,
    pub validate_no_gc: &'static str,
    pub validate_removed: &'static str,
    pub validate_remove_fail: &'static str,
    pub validate_none: &'static str,
    pub validate_done: &'static str,

    // ---- gcup unpack (invoked from Validate) ----
    pub gcup_unpack: &'static str,
    pub gcup_unpack_ok: &'static str,
    pub gcup_unpack_fail: &'static str,
    pub gcup_archive_missing: &'static str,
    pub gcup_extracted: &'static str,
    pub gcup_files: &'static str,

    pub fix_icons: &'static str,
    pub no_econ: &'static str,
    pub rd_ok: &'static str,
    pub rd_fail: &'static str,
    pub backup_ok: &'static str,
    pub backup_fail: &'static str,

    pub fix_infinite: &'static str,
    pub copy_hosts: &'static str,
    pub hosts_ok: &'static str,
    pub hosts_nochange: &'static str,
    pub hosts_notfound: &'static str,
    pub hosts_fail: &'static str,
    pub hosts_warn: &'static str,
    pub noservice: &'static str,
    pub launching: &'static str,

    pub av_check: &'static str,
    pub defender_ok: &'static str,
    pub defender_fail: &'static str,
    pub av_found: &'static str,
    pub av_manual: &'static str,
    pub av_hdr: &'static str,
    pub av_p1: &'static str,
    pub av_p2: &'static str,
    pub av_noav: &'static str,

    pub dns_header: &'static str,
    pub dns_adapter: &'static str,
    pub dns_noadapter: &'static str,
    pub dns_set: &'static str,
    pub dns_doh_try: &'static str,
    pub dns_doh_ok: &'static str,
    pub dns_doh_unsup: &'static str,
    pub dns_flush: &'static str,
    pub dns_done: &'static str,

    pub tut_header: &'static str,
    pub tut_intro: &'static str,
    pub tut_step1_title: &'static str,
    pub tut_step1_verify: &'static str,
    pub tut_step1_hint: &'static str,
    pub tut_step2_title: &'static str,
    pub tut_step2_body: &'static str,
    pub tut_step3_title: &'static str,
    pub tut_step3_body: &'static str,
    pub tut_step3_sample: &'static str,
    pub tut_step4_title: &'static str,
    pub tut_step4_body: &'static str,
    pub tut_step4_body2: &'static str,
    pub tut_step5_title: &'static str,
    pub tut_step5_body: &'static str,
    pub tut_step6_title: &'static str,
    pub tut_step6_body: &'static str,
    pub tut_footer: &'static str,
    pub tut_warn_red: &'static str,

    // ---- GUI labels ----
    pub gui_language_label: &'static str,
    pub gui_folder_label: &'static str,
    pub gui_browse: &'static str,
    pub gui_working: &'static str,
    pub gui_continue: &'static str,
    pub gui_continue_title: &'static str,
    pub gui_log_heading: &'static str,
    pub gui_path_empty: &'static str,
    pub gui_invalid_prefix: &'static str,
    pub gui_error_prefix: &'static str,
    pub gui_tutorial_opened: &'static str,
    pub gui_tutorial_open_failed: &'static str,

    // ---- Buttons ----
    pub menu1: &'static str,
    pub menu2: &'static str,
    pub menu3: &'static str,
    pub menu4: &'static str,
    pub menu6: &'static str,
    pub menu7: &'static str,

    pub psdesc: &'static str,
}

pub const EN: Messages = Messages {
    title: "CS:GO Legacy Fixer",

    killing: "Closing Steam and CS:GO processes...",
    killing_csgo: "Closing CS:GO processes...",

    fix_update: "Preparing update fix...",
    downloading: "Downloading update.gcup...",
    download_fail: "Failed to download update.gcup.",
    hash_ok: "File is not corrupted.",
    hash_fail: "Size or SHA256 mismatch!",
    manifest_fail: "Failed to download manifest from all mirrors.",
    manifest_incomplete: "Manifest is missing size/sha256 - refusing to install (fail-closed).",
    sig_missing: "Manifest has no 'sig' field.",
    sig_created: "Created update.gcup.sig.",
    copy_ok: "Copied update.gcup and update.gcup.sig to game root.",
    copy_fail: "Failed to copy update files to game root.",
    update_done: "Update fix applied successfully. You can launch the game.",
    disk_low: "Not enough free disk space on the game drive.",
    version: "Version: ",
    size: "Size: ",

    sig_verified: "Ed25519 signature verified.",
    sig_skip_no_key: "No Ed25519 public key found — signature check skipped.",
    sig_fail: "Ed25519 signature INVALID — rejecting update (possible tampering).",
    sig_bad_key: "Public key file is malformed: ",

    cancel: "Cancel",
    cancelling: "Cancelling…",
    cancelled: "Cancelled by user.",

    fix_validate: "Starting Steam integrity check for file-mismatch kick...",
    validate_wait: "Steam validation started. Wait until it finishes, then press Continue.",
    validate_no_gc: "csgo_gc folder not found: ",
    validate_removed: "Removed: ",
    validate_remove_fail: "Failed to remove: ",
    validate_none: "No .content_*.state files found.",
    validate_done: "Validation cleanup done. Running update fix...",

    gcup_unpack: "Unpacking update.gcup...",
    gcup_unpack_ok: "update.gcup unpacked.",
    gcup_unpack_fail: "Failed to unpack update.gcup.",
    gcup_archive_missing: "update.gcup not found: ",
    gcup_extracted: "Extracted: ",
    gcup_files: "Files extracted: ",

    fix_icons: "Removing econ folder...",
    no_econ: "econ folder not found: ",
    rd_ok: "Deleted econ folder.",
    rd_fail: "Could not delete econ folder.",
    backup_ok: "Backup created: ",
    backup_fail: "Backup failed for: ",

    fix_infinite: "Infinite 'Connecting to CS:GO network...' fix",
    copy_hosts: "Appending hosts into drivers\\etc ...",
    hosts_ok: "Merged: ",
    hosts_nochange: "No new lines to add: ",
    hosts_notfound: "Source file not found in script folder: ",
    hosts_fail: "Merge failed: ",
    hosts_warn: "Some files could not be merged. Check that you are running as Administrator.",
    noservice: "resources\\zaprettt\\service.bat not found next to this script.",
    launching: "Launching resources\\zaprettt\\service.bat ...",

    av_check: "Checking antivirus exclusions...",
    defender_ok: "Added hosts to Windows Defender exclusions.",
    defender_fail: "Could not add Windows Defender exclusions (Defender may not be active or you may lack rights).",
    av_found: "Detected antivirus product(s):",
    av_manual: "Please add these files to your antivirus exclusion list manually:",
    av_hdr: "  Excluded files:",
    av_p1: "    %SystemRoot%\\System32\\drivers\\etc\\hosts",
    av_p2: "",
    av_noav: "No antivirus products reported by Windows Security Center.",

    dns_header: "Configuring Cloudflare DNS and Secure DNS (DoH)...",
    dns_adapter: "Active adapter: ",
    dns_noadapter: "Could not detect active network adapter. Skipping DNS configuration.",
    dns_set: "Applying Cloudflare DNS (1.1.1.1 / 1.0.0.1 + IPv6)...",
    dns_doh_try: "Secure DNS (DoH) is supported, enabling...",
    dns_doh_ok: "Secure DNS (DoH) enabled successfully.",
    dns_doh_unsup: "Secure DNS (DoH) is not supported by this Windows version.",
    dns_flush: "DNS cache flushed.",
    dns_done: "DNS configuration complete.",

    tut_header: "service.bat tutorial",
    tut_intro: "A second window (service.bat) just opened. Follow the steps in order.",
    tut_step1_title: "Step 1 - Check the settings",
    tut_step1_verify: "In the service.bat window, make sure these two lines look like this:",
    tut_step1_hint: "If they show anything else, change them to match.",
    tut_step2_title: "Step 2 - Reset",
    tut_step2_body: "Select option 2, press Enter. Wait for it to finish, then press any key when it asks.",
    tut_step3_title: "Step 3 - Run the test",
    tut_step3_body: "Select option 12. A new window opens. Select 1, then 1 again. Wait until all tests finish.",
    tut_step3_sample: "You will see a report like this at the end:",
    tut_step4_title: "Step 4 - Remember the best config",
    tut_step4_body: "Write down the name shown next to 'Best config:' (for example: general (EXP).bat).",
    tut_step4_body2: "Press any key in the new window, then any key in the original service.bat window.",
    tut_step5_title: "Step 5 - Apply the best config",
    tut_step5_body: "Back in service.bat, press 1. Find the same config name in the list and press its number.",
    tut_step6_title: "Step 6 - Finish",
    tut_step6_body: "Close the service.bat window. You are done.",
    tut_footer: "If something goes wrong, close service.bat and start this fix again.",
    tut_warn_red: "WARNING: Do NOT delete or move the resources\\zaprettt folder!",

    gui_language_label: "Language:",
    gui_folder_label: "Game folder:",
    gui_browse: "Browse…",
    gui_working: "Working…",
    gui_continue: "Continue",
    gui_continue_title: "Action required",
    gui_log_heading: "Log",
    gui_path_empty: "Game path is empty.",
    gui_invalid_prefix: "Invalid path: ",
    gui_error_prefix: "Error: ",
    gui_tutorial_opened: "Tutorial opened in Notepad.",
    gui_tutorial_open_failed: "Could not open the tutorial in Notepad. File left at: ",

    menu1: "Game doesn't update",
    menu2: "Inventory icons are messed up",
    menu3: "Infinite 'Connecting to CS:GO network...'",
    menu4: "Kicked from server (file mismatch)",
    menu6: "I have another problem / the script didn't fix it",
    menu7: "Exit",

    psdesc: "Select the CS:GO Legacy folder",
};

pub const RU: Messages = Messages {
    title: "CS:GO Legacy Fixer",

    killing: "Закрытие процессов Steam и CS:GO...",
    killing_csgo: "Закрытие процессов CS:GO...",

    fix_update: "Подготовка исправления обновления...",
    downloading: "Скачивание update.gcup...",
    download_fail: "Не удалось скачать update.gcup.",
    hash_ok: "Файл не повреждён.",
    hash_fail: "Размер или SHA256 не совпадают!",
    manifest_fail: "Не удалось скачать манифест ни с одного зеркала.",
    manifest_incomplete: "В манифесте нет size/sha256 — установка отменена (fail-closed).",
    sig_missing: "В манифесте нет поля 'sig'.",
    sig_created: "Создан update.gcup.sig.",
    copy_ok: "Файлы update.gcup и update.gcup.sig скопированы в корень игры.",
    copy_fail: "Не удалось скопировать файлы обновления в корень игры.",
    update_done: "Исправление обновления применено успешно. Можно запускать игру.",
    disk_low: "Недостаточно свободного места на диске игры.",
    version: "Версия: ",
    size: "Размер: ",

    sig_verified: "Подпись Ed25519 проверена.",
    sig_skip_no_key: "Публичный ключ Ed25519 не найден — проверка подписи пропущена.",
    sig_fail: "Подпись Ed25519 НЕВЕРНА — обновление отклонено (возможно вмешательство).",
    sig_bad_key: "Файл публичного ключа повреждён: ",

    cancel: "Отмена",
    cancelling: "Отмена…",
    cancelled: "Отменено пользователем.",

    fix_validate: "Запуск проверки целостности Steam для ошибки 'файл не совпадает'...",
    validate_wait: "Проверка Steam запущена. Дождитесь завершения и нажмите «Продолжить».",
    validate_no_gc: "Папка csgo_gc не найдена: ",
    validate_removed: "Удалён: ",
    validate_remove_fail: "Не удалось удалить: ",
    validate_none: "Файлы .content_*.state не найдены.",
    validate_done: "Очистка после проверки завершена. Запуск исправления обновления...",

    gcup_unpack: "Распаковка update.gcup...",
    gcup_unpack_ok: "update.gcup распакован.",
    gcup_unpack_fail: "Не удалось распаковать update.gcup.",
    gcup_archive_missing: "Файл update.gcup не найден: ",
    gcup_extracted: "Извлечён: ",
    gcup_files: "Извлечено файлов: ",

    fix_icons: "Удаление папки econ...",
    no_econ: "Папка econ не найдена: ",
    rd_ok: "Папка econ удалена.",
    rd_fail: "Не удалось удалить папку econ.",
    backup_ok: "Создан бэкап: ",
    backup_fail: "Не удалось создать бэкап: ",

    fix_infinite: "Исправление бесконечного 'Подключение к сети CS:GO...'",
    copy_hosts: "Добавление hosts в drivers\\etc ...",
    hosts_ok: "Объединено: ",
    hosts_nochange: "Нет новых строк для добавления: ",
    hosts_notfound: "Исходный файл не найден в папке скрипта: ",
    hosts_fail: "Не удалось объединить: ",
    hosts_warn: "Некоторые файлы не удалось обработать. Проверьте, что скрипт запущен от имени администратора.",
    noservice: "Файл resources\\zaprettt\\service.bat не найден рядом со скриптом.",
    launching: "Запуск resources\\zaprettt\\service.bat ...",

    av_check: "Проверка исключений антивируса...",
    defender_ok: "Файл hosts добавлен в исключения Windows Defender.",
    defender_fail: "Не удалось добавить исключения Windows Defender (Defender неактивен или недостаточно прав).",
    av_found: "Обнаруженные антивирусные продукты:",
    av_manual: "Пожалуйста, добавьте следующие файлы в исключения антивируса вручную:",
    av_hdr: "  Файлы для исключения:",
    av_p1: "    %SystemRoot%\\System32\\drivers\\etc\\hosts",
    av_p2: "",
    av_noav: "Windows Security Center не сообщает об установленных антивирусах.",

    dns_header: "Настройка Cloudflare DNS и защищенного DNS (DoH)...",
    dns_adapter: "Активный адаптер: ",
    dns_noadapter: "Не удалось определить активный сетевой адаптер. Пропускаю настройку DNS.",
    dns_set: "Применяю Cloudflare DNS (1.1.1.1 / 1.0.0.1 + IPv6)...",
    dns_doh_try: "Secure DNS (DoH) поддерживается, включаю...",
    dns_doh_ok: "Secure DNS (DoH) успешно включен.",
    dns_doh_unsup: "Secure DNS (DoH) не поддерживается этой версией Windows.",
    dns_flush: "DNS-кэш очищен.",
    dns_done: "Настройка DNS завершена.",

    tut_header: "Инструкция к service.bat",
    tut_intro: "Открылось второе окно (service.bat). Выполняйте шаги по порядку.",
    tut_step1_title: "Шаг 1 — Проверьте настройки",
    tut_step1_verify: "В окне service.bat убедитесь, что эти две строки выглядят так:",
    tut_step1_hint: "Если там другие значения — поменяйте на эти.",
    tut_step2_title: "Шаг 2 — Сброс",
    tut_step2_body: "Выберите пункт 2, нажмите Enter. Дождитесь завершения, затем нажмите любую клавишу.",
    tut_step3_title: "Шаг 3 — Запустите тест",
    tut_step3_body: "Выберите пункт 12. Откроется новое окно. Выберите 1, затем снова 1. Дождитесь окончания всех тестов.",
    tut_step3_sample: "В конце вы увидите отчёт такого вида:",
    tut_step4_title: "Шаг 4 — Запомните лучший конфиг",
    tut_step4_body: "Запишите название из строки 'Best config:' (например: general (EXP).bat).",
    tut_step4_body2: "Нажмите любую клавишу в новом окне, затем любую клавишу в исходном окне service.bat.",
    tut_step5_title: "Шаг 5 — Примените лучший конфиг",
    tut_step5_body: "Вернитесь в service.bat, нажмите 1. Найдите тот же конфиг в списке и нажмите его номер.",
    tut_step6_title: "Шаг 6 — Завершение",
    tut_step6_body: "Закройте окно service.bat. Готово.",
    tut_footer: "Если что-то пошло не так — закройте service.bat и запустите исправление заново.",
    tut_warn_red: "ВНИМАНИЕ: НЕЛЬЗЯ удалять или перемещать папку resources\\zaprettt!",

    gui_language_label: "Язык:",
    gui_folder_label: "Папка игры:",
    gui_browse: "Обзор…",
    gui_working: "Работаю…",
    gui_continue: "Продолжить",
    gui_continue_title: "Требуется действие",
    gui_log_heading: "Журнал",
    gui_path_empty: "Путь к игре не задан.",
    gui_invalid_prefix: "Неверный путь: ",
    gui_error_prefix: "Ошибка: ",
    gui_tutorial_opened: "Инструкция открыта в Блокноте.",
    gui_tutorial_open_failed: "Не удалось открыть инструкцию в Блокноте. Файл сохранён: ",

    menu1: "Игра не обновляется",
    menu2: "Иконки инвентаря сломаны",
    menu3: "Бесконечное 'Подключение к сети CS:GO...'",
    menu4: "Кикает с сервера (файл не совпадает)",
    menu6: "У меня другая проблема / скрипт не исправил её",
    menu7: "Выход",

    psdesc: "Выберите папку CS:GO Legacy",
};

pub fn messages(lang: Lang) -> &'static Messages {
    match lang {
        Lang::En => &EN,
        Lang::Ru => &RU,
    }
}

pub fn init_logging() -> Result<()> {
    let base: PathBuf = std::env::current_exe()
        .ok().and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    let log_dir = base.join("logs");
    std::fs::create_dir_all(&log_dir).context("create log dir")?;

    let file_appender = tracing_appender::rolling::daily(&log_dir, "fixer.log");

    let level = std::env::var("RUST_LOG")
        .ok()
        .and_then(|s| s.parse::<tracing::Level>().ok())
        .unwrap_or(tracing::Level::INFO);

    let _ = tracing_subscriber::fmt()
        .with_writer(file_appender)
        .with_max_level(level)
        .with_ansi(false)
        .try_init();

    Ok(())
}