<h1 align="center">Taskathlon</h1>

<p align="center">
  <b>Тренажёр скорости и собеседований по Excel, SQL и бухучёту.</b><br>
  Короткие уроки, рабочие кейсы, экспресс-тесты на время и спринты на рекорд.<br>
  Внутри настоящий табличный редактор, а не картинка.
</p>

<p align="center">
  <img alt="Rust" src="https://img.shields.io/badge/written%20in-Rust-0f766e?style=flat-square&logo=rust&logoColor=white">
  <img alt="License: MIT OR Apache-2.0" src="https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-0f766e?style=flat-square">
  <img alt="Built on GridCraft" src="https://img.shields.io/badge/built%20on-GridCraft-2563eb?style=flat-square">
</p>

<p align="center">
  <img src="docs/images/taskathlon-home.png" alt="Главный экран: путь из разделов, цель дня, задания дня, экспресс-тест" width="100%">
</p>

## Зачем

Для тех, кто идёт на бухгалтера, офис-аналитика или финансиста и хочет, чтобы Excel и SQL работали «на автомате». Можно за день подготовиться к онлайн-тесту при отборе.

## Что внутри

- 🧮 **Excel по-русски.** `=СУММЕСЛИ(A2:A9;"Самара";B2:B9)`, `$`, ВПР, ПРОСМОТРX, ИНДЕКС+ПОИСКПОЗ, даты, текст, сводные и горячие клавиши. Формулы считает движок GridCraft.
- 🗄 **SQL.** 16 навыков от `SELECT` до оконных функций: пишешь запрос, видишь результат. Проверка идёт на других данных, поэтому подогнать ответ не получится.
- 📒 **Бухучёт.** План счетов, проводки, НДС, зарплата, ОСВ.
- 💼 **Рабочие кейсы и режим «Собеседование»:** письмо от руководителя, данные, таймер, разбор.
- ⏱ **Экспресс-тест:** 15 вопросов, у каждого свой таймер. Excel, SQL или микс.
- 🏁 **Спринт:** 10 заданий на секундомер, медали и личные рекорды.
- ⚡ **Мотивация:** чем быстрее ответ, тем больше очков. Комбо, серия дней, цель дня, задания дня, кристаллы, заморозка серии, ставки на серию и достижения.
- 🧠 **Адаптивность:** то, что уже знаешь, проходится быстрее («Я это знаю», перескок уровня). Ошибки возвращаются, а повторения идут по FSRS.
- 🔔 **Напоминание** на рабочем столе через таймер systemd. Всё работает офлайн.

<p align="center">
  <img src="docs/images/taskathlon-sql.png" alt="SQL-задание" width="49%">
  <img src="docs/images/taskathlon-blitz.png" alt="Экспресс-тест" width="49%">
</p>

## Запуск

Нужен Rust 1.95+ ([rustup](https://rustup.rs)).

```sh
cargo run --release -p gridcraft-learn --bin taskathlon
```

Нажми **Enter**, и урок начнётся. Прогресс сохраняется сам в `~/.local/share/gridcraft-learn/`.

## Основано на GridCraft

Taskathlon построен на **[GridCraft](https://github.com/storytold/gridcraft)**, открытом табличном редакторе на Rust от команды ArtCraft (MIT OR Apache-2.0). Движок формул, таблица и интерфейс ячеек взяты оттуда. Код тренажёра лежит в `crates/learn` и `apps/gridcraft-learn`, а исходный редактор GridCraft собирается как `cargo run -p gridcraft`.

Taskathlon не является продуктом ArtCraft, и команда ArtCraft его не одобряла и не поддерживает. Что изменено относительно GridCraft, перечислено в [NOTICE](NOTICE).

## Права

- Лицензия: MIT или Apache-2.0 на выбор ([LICENSE-MIT](LICENSE-MIT), [LICENSE-APACHE](LICENSE-APACHE)).
- Весь контент, маскот, звуки и цвета оригинальные. Ассеты сторонних вендоров не используются ([ATTRIBUTION.md](ATTRIBUTION.md)).
- Excel является товарным знаком Microsoft, 1С является товарным знаком фирмы «1С». Taskathlon с ними не связан и не содержит их кода, иконок или шаблонов.

## Документация

[Как устроено](docs/learn/ARCHITECTURE.md) · [Адаптивная модель](docs/learn/ADAPTIVE_MODEL.md) · [Ограничения](docs/learn/KNOWN_LIMITATIONS.md) · [Прогресс](docs/learn/PROGRESS.md)
