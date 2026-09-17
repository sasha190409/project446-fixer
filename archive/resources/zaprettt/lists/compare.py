import os
from pathlib import Path

def main():
    # Текущая директория (D:\zaprettt\lists) – здесь находится папка new
    lists_dir = Path('.')
    new_dir = lists_dir / 'new'
    compared_dir = new_dir / 'compared'

    if not new_dir.exists():
        print(f"Папка {new_dir} не найдена.")
        return

    # Создаём папку для результатов, если её нет
    compared_dir.mkdir(exist_ok=True)

    # Получаем список файлов в new (только файлы, не папки)
    files = [f for f in new_dir.iterdir() if f.is_file()]

    if not files:
        print("В папке new нет файлов.")
        return

    processed = 0
    for new_file in files:
        filename = new_file.name
        lists_file = lists_dir / filename

        # Читаем содержимое файла из new
        try:
            with open(new_file, 'r', encoding='utf-8', errors='ignore') as f:
                new_lines = f.readlines()
        except Exception as e:
            print(f"Не удалось прочитать {new_file}: {e}")
            continue

        # Если файл есть в lists – читаем его, иначе считаем все строки новыми
        if lists_file.exists():
            try:
                with open(lists_file, 'r', encoding='utf-8', errors='ignore') as f:
                    lists_lines = f.readlines()
            except Exception as e:
                print(f"Не удалось прочитать {lists_file}: {e}")
                continue
            # Множество строк из lists (без учёта пробелов в начале/конце)
            lists_set = set(line.strip() for line in lists_lines)
        else:
            lists_set = set()

        # Находим уникальные новые строки, сохраняя порядок
        diff_lines = []
        seen = set()
        for line in new_lines:
            stripped = line.strip()
            # Игнорируем пустые строки (уберите `stripped and` если нужно их учитывать)
            if stripped and stripped not in lists_set and stripped not in seen:
                diff_lines.append(line)
                seen.add(stripped)

        if diff_lines:
            compared_file = compared_dir / filename
            try:
                with open(compared_file, 'w', encoding='utf-8') as f:
                    f.writelines(diff_lines)
                print(f"Файл {filename}: найдено {len(diff_lines)} новых строк, записаны в {compared_file}")
            except Exception as e:
                print(f"Ошибка записи {compared_file}: {e}")
        else:
            print(f"Файл {filename}: новых строк не найдено.")

        processed += 1

    print(f"Обработано {processed} файлов. Результаты в {compared_dir}")

if __name__ == "__main__":
    main()