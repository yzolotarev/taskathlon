#!/usr/bin/env bash
# Запускает свежую сборку Taskathlon из этого каталога.
# Предпочитает release, иначе использует debug. Перед релизом можно
# пересобрать:  cargo build --release -p gridcraft-learn
set -euo pipefail
dir="$(cd "$(dirname "$0")" && pwd)"

if [ -x "$dir/target/release/taskathlon" ]; then
  exec "$dir/target/release/taskathlon" "$@"
elif [ -x "$dir/target/debug/taskathlon" ]; then
  echo "Taskathlon: релизная сборка не найдена, запускаю debug." >&2
  exec "$dir/target/debug/taskathlon" "$@"
else
  echo "Taskathlon не собран. Выполните: cargo build --release -p gridcraft-learn" >&2
  exit 1
fi
