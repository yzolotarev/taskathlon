//! "Учёт": Taskathlon's own accounting program on top of `gridcraft-ledger`. The layout is
//! original: sections on the left, a journal of documents, document forms with posting,
//! entries, reports and catalogs.

use crate::view::{Palette, big_button, ghost_button};
use egui::{Align, Color32, FontId, Layout, RichText, Ui, vec2};
use gridcraft_ledger::catalog::Vat;
use gridcraft_ledger::document::{self, Body, ExpenseLine, GoodsLine, OpeningLine};
use gridcraft_ledger::practice::{self, Task};
use gridcraft_ledger::report;
use gridcraft_ledger::{Book, Date, DocKind, Document, Money, Sub, accounts};
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Section {
    Money,
    Purchases,
    Sales,
    Stock,
    People,
    MonthEnd,
    All,
    Reports,
    Catalogs,
}

impl Section {
    const ALL: [Section; 9] = [
        Section::Money,
        Section::Purchases,
        Section::Sales,
        Section::Stock,
        Section::People,
        Section::MonthEnd,
        Section::All,
        Section::Reports,
        Section::Catalogs,
    ];

    fn title(self) -> &'static str {
        match self {
            Section::Money => "💰 Деньги",
            Section::Purchases => "⬇ Закупки",
            Section::Sales => "⬆ Продажи",
            Section::Stock => "📦 Склад",
            Section::People => "👥 Сотрудники",
            Section::MonthEnd => "🏁 Конец месяца",
            Section::All => "📒 Все документы",
            Section::Reports => "📊 Отчёты",
            Section::Catalogs => "📚 Справочники",
        }
    }

    fn kinds(self) -> &'static [DocKind] {
        match self {
            Section::Money => &[DocKind::BankIn, DocKind::BankOut, DocKind::CashIn, DocKind::CashOut],
            Section::Purchases => &[DocKind::Receipt, DocKind::Services, DocKind::AssetPurchase],
            Section::Sales => &[DocKind::Sale],
            Section::Stock => &[DocKind::WriteOff],
            Section::People => &[DocKind::Payroll, DocKind::AdvanceReport],
            Section::MonthEnd => &[DocKind::MonthClose],
            Section::All => &DocKind::ALL,
            Section::Reports | Section::Catalogs => &[],
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ReportKind {
    Trial,
    Card,
    Analysis,
    Stock,
    Results,
}

impl ReportKind {
    const ALL: [ReportKind; 5] = [ReportKind::Trial, ReportKind::Card, ReportKind::Analysis, ReportKind::Stock, ReportKind::Results];
    fn title(self) -> &'static str {
        match self {
            ReportKind::Trial => "Оборотно-сальдовая ведомость",
            ReportKind::Card => "Карточка счёта",
            ReportKind::Analysis => "Анализ счёта",
            ReportKind::Stock => "Остатки товаров",
            ReportKind::Results => "Финансовый результат",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CatalogKind {
    Counterparties,
    Items,
    Employees,
    Assets,
}

/// A running workshop: tasks done one after another against the clock.
pub struct Practice {
    pub tasks: Vec<Task>,
    pub index: usize,
    pub started: std::time::Instant,
    pub began: std::time::Instant,
    pub answer: String,
    pub error: Option<String>,
    pub show_hint: bool,
    /// Seconds per solved task (None = skipped).
    pub results: Vec<Option<f32>>,
}

/// What happened on screen this frame, for the trainer around the program.
#[derive(Default)]
pub struct Outcome {
    pub back: bool,
    /// A task was solved: seconds taken and par.
    pub solved: Option<(f32, u32)>,
    pub failed: bool,
    /// The workshop ended: solved count and total.
    pub finished: Option<(usize, usize)>,
}

pub struct LedgerView {
    pub book: Book,
    pub section: Section,
    pub selected: Option<u32>,
    /// The open document form: a working copy and whether it was just created.
    pub draft: Option<(Document, bool)>,
    pub show_entries: bool,
    pub report: ReportKind,
    pub account: String,
    /// Report period: first and last month (1–12) of 2026.
    pub from_month: u8,
    pub to_month: u8,
    pub message: Option<(bool, String)>,
    pub practice: Option<Practice>,
    catalog: CatalogKind,
    new_name: String,
    new_extra: String,
    path: Option<PathBuf>,
}

impl LedgerView {
    /// Loads the saved books next to the progress file, or the demo company.
    pub fn load(path: Option<PathBuf>) -> LedgerView {
        let saved = path.as_ref().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|s| Book::from_json(&s).ok());
        let book = saved.or_else(|| gridcraft_ledger::demo::january().ok()).unwrap_or_else(|| Book::new("Моя организация"));
        LedgerView {
            book,
            section: Section::All,
            selected: None,
            draft: None,
            show_entries: false,
            report: ReportKind::Trial,
            account: "60".into(),
            from_month: 1,
            to_month: 1,
            message: None,
            practice: None,
            catalog: CatalogKind::Counterparties,
            new_name: String::new(),
            new_extra: String::new(),
            path,
        }
    }

    /// Starts the February workshop on a fresh copy of the demo company.
    pub fn start_practice(&mut self, seed: u64) {
        if let Ok(b) = gridcraft_ledger::demo::january() {
            self.book = b;
        }
        self.draft = None;
        self.selected = None;
        self.from_month = 2;
        self.to_month = 2;
        self.section = Section::All;
        let now = crate::instant();
        self.practice = Some(Practice {
            tasks: practice::february(seed),
            index: 0,
            started: now,
            began: now,
            answer: String::new(),
            error: None,
            show_hint: false,
            results: Vec::new(),
        });
        self.message = Some((true, "Практикум начат: демо-компания, январь закрыт. Работаем в феврале.".into()));
        self.save();
    }

    fn save(&mut self) {
        let Some(path) = &self.path else { return };
        match self.book.to_json() {
            Ok(s) => {
                if let Err(e) = std::fs::write(path, s) {
                    self.message = Some((false, format!("Не удалось сохранить учёт: {e}")));
                }
            }
            Err(e) => self.message = Some((false, e.to_string())),
        }
    }

    fn period(&self) -> (Date, Date) {
        let a = Date { y: 2026, m: self.from_month.clamp(1, 12), d: 1 };
        let b = Date { y: 2026, m: self.to_month.clamp(1, 12).max(a.m), d: 1 }.month_end();
        (a, b)
    }

    /// A fresh document of `kind` with sensible defaults, added unposted.
    fn create(&mut self, kind: DocKind) {
        let c = &self.book.catalogs;
        let cp = c.counterparties.first().map(|x| x.id).unwrap_or(0);
        let item = c.items.first().map(|x| x.id).unwrap_or(0);
        let emp = c.employees.first().map(|x| x.id).unwrap_or(0);
        let asset = c.assets.first().map(|x| x.id).unwrap_or(0);
        let body = match kind {
            DocKind::Opening => {
                Body::Opening { lines: vec![OpeningLine { account: "51".into(), sub: None, debit: Money::ZERO, credit: Money::ZERO, qty: 0 }] }
            }
            DocKind::Receipt => Body::Receipt { supplier: cp, account: "41".into(), lines: vec![GoodsLine { item, qty: 1, amount: Money::ZERO }] },
            DocKind::Services => Body::Services {
                supplier: cp,
                lines: vec![ExpenseLine { text: "Услуги".into(), amount: Money::ZERO, vat: Vat::R22, account: "26".into() }],
            },
            DocKind::Sale => Body::Sale { customer: cp, lines: vec![GoodsLine { item, qty: 1, amount: Money::ZERO }] },
            DocKind::AssetPurchase => Body::AssetPurchase { supplier: cp, asset, amount: Money::ZERO, vat: Vat::R22 },
            DocKind::WriteOff => Body::WriteOff { account: "26".into(), lines: vec![GoodsLine { item, qty: 1, amount: Money::ZERO }] },
            DocKind::BankIn => Body::BankIn { from: document::BankIn::Customer(cp), amount: Money::ZERO },
            DocKind::BankOut => Body::BankOut { to: document::BankOut::Supplier(cp), amount: Money::ZERO },
            DocKind::CashIn => Body::CashIn { from: document::CashIn::Customer(cp), amount: Money::ZERO },
            DocKind::CashOut => Body::CashOut { to: document::CashOut::Advance(emp), amount: Money::ZERO },
            DocKind::AdvanceReport => Body::AdvanceReport {
                employee: emp,
                lines: vec![ExpenseLine { text: "Расходы".into(), amount: Money::ZERO, vat: Vat::None, account: "26".into() }],
            },
            DocKind::Payroll => Body::Payroll { account: "26".into(), lines: c.employees.iter().map(|e| (e.id, e.salary)).collect() },
            DocKind::MonthClose => Body::MonthClose,
        };
        let (_, to) = self.period();
        let date = if kind == DocKind::MonthClose { to } else { Date { d: to.d.min(15), ..to } };
        let id = self.book.add(date, body);
        if let Some(d) = self.book.doc(id).cloned() {
            self.selected = Some(id);
            self.draft = Some((d, true));
        }
    }

    /// Writes the form back; with `post`, posts it too. Returns whether the form may close.
    fn write(&mut self, post: bool) -> bool {
        let Some((draft, _)) = self.draft.clone() else { return true };
        let id = draft.id;
        let was_posted = self.book.doc(id).is_some_and(|d| d.posted);
        if let Some(d) = self.book.doc_mut(id) {
            d.date = draft.date;
            d.body = draft.body.clone();
            d.comment = draft.comment.clone();
        }
        let result = if post || was_posted { self.book.post(id) } else { Ok(()) };
        self.save();
        match result {
            Ok(()) => {
                self.message =
                    Some((true, if post || was_posted { format!("{} проведён", draft.number) } else { format!("{} записан", draft.number) }));
                if let Some((d, fresh)) = self.draft.as_mut() {
                    d.posted = post || was_posted;
                    *fresh = false;
                }
                true
            }
            Err(e) => {
                self.message = Some((false, format!("Не удалось провести: {e}")));
                false
            }
        }
    }

    fn close_form(&mut self) {
        if let Some((d, true)) = &self.draft {
            let _ = self.book.remove(d.id);
        }
        self.draft = None;
    }

    fn party(&self, d: &Document) -> String {
        let c = &self.book.catalogs;
        let cp = |id: u32| c.counterparty(id).map(|x| x.name.clone()).unwrap_or_default();
        let emp = |id: u32| c.employee(id).map(|x| x.name.clone()).unwrap_or_default();
        match &d.body {
            Body::Receipt { supplier, .. } | Body::Services { supplier, .. } | Body::AssetPurchase { supplier, .. } => cp(*supplier),
            Body::Sale { customer, .. } => cp(*customer),
            Body::BankIn { from, .. } => match from {
                document::BankIn::Customer(id) | document::BankIn::Loan(id) => cp(*id),
                document::BankIn::FromCash => "Из кассы".into(),
            },
            Body::BankOut { to, .. } => match to {
                document::BankOut::Supplier(id) | document::BankOut::LoanRepay(id) => cp(*id),
                document::BankOut::Salary(id) => emp(*id),
                document::BankOut::Tax(acc) => format!("Налог ({acc})"),
                document::BankOut::Insurance => "Страховые взносы".into(),
                document::BankOut::Fee => "Комиссия банка".into(),
                document::BankOut::ToCash => "В кассу".into(),
            },
            Body::CashIn { from, .. } => match from {
                document::CashIn::Customer(id) => cp(*id),
                document::CashIn::AdvanceReturn(id) => emp(*id),
            },
            Body::CashOut { to, .. } => match to {
                document::CashOut::Advance(id) | document::CashOut::Salary(id) => emp(*id),
                document::CashOut::Supplier(id) => cp(*id),
            },
            Body::AdvanceReport { employee, .. } => emp(*employee),
            Body::Payroll { lines, .. } => format!("{} сотр.", lines.len()),
            Body::WriteOff { account, .. } => format!("на счёт {account}"),
            Body::Opening { .. } | Body::MonthClose => String::new(),
        }
    }
}

const MONTHS: [&str; 12] = ["Январь", "Февраль", "Март", "Апрель", "Май", "Июнь", "Июль", "Август", "Сентябрь", "Октябрь", "Ноябрь", "Декабрь"];

fn month_name(m: u8) -> &'static str {
    MONTHS.get(usize::from(m.clamp(1, 12)) - 1).copied().unwrap_or("")
}

/// Draws the whole program. Returns true when the user wants to go back to the trainer.
pub fn show(v: &mut LedgerView, ui: &mut Ui, p: &Palette, seed: u64) -> Outcome {
    let mut out = Outcome::default();
    let mut back = false;
    egui::Panel::top("ledger_top").frame(egui::Frame::NONE.fill(p.card).inner_margin(egui::Margin::symmetric(16, 10))).show(ui, |ui| {
        ui.horizontal(|ui| {
            if ui.button("← К тренажёру").clicked() {
                back = true;
            }
            ui.add_space(8.0);
            ui.label(RichText::new("📒 Учёт").size(22.0).strong().color(p.text));
            ui.label(RichText::new(&v.book.org).size(16.0).color(p.muted));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if v.practice.is_none() && ui.button("🎯 Практикум: февраль").on_hover_text("11 заданий на время на демо-компании").clicked()
                {
                    v.start_practice(seed);
                }
                if ui.button("Начать заново (демо)").on_hover_text("Вернуть демо-компанию: январь с проведёнными документами").clicked()
                    && let Ok(b) = gridcraft_ledger::demo::january()
                {
                    v.book = b;
                    v.draft = None;
                    v.selected = None;
                    v.practice = None;
                    v.save();
                    v.message = Some((true, "Демо-компания восстановлена".into()));
                }
                let (from, to) = (v.from_month, v.to_month);
                egui::ComboBox::from_id_salt("ledger_to").selected_text(month_name(to)).show_ui(ui, |ui| {
                    for m in from..=12 {
                        ui.selectable_value(&mut v.to_month, m, month_name(m));
                    }
                });
                ui.label("—");
                egui::ComboBox::from_id_salt("ledger_from").selected_text(month_name(from)).show_ui(ui, |ui| {
                    for m in 1..=12 {
                        ui.selectable_value(&mut v.from_month, m, month_name(m));
                    }
                });
                v.to_month = v.to_month.max(v.from_month);
                ui.label(RichText::new("Период 2026:").color(p.muted));
            });
        });
    });
    if v.practice.is_some() {
        practice_bar(v, ui, p, &mut out);
    }
    egui::Panel::left("ledger_sections").exact_size(200.0).frame(egui::Frame::NONE.fill(p.card).inner_margin(egui::Margin::same(10))).show(
        ui,
        |ui| {
            for s in Section::ALL {
                let on = v.section == s;
                let text = RichText::new(s.title()).size(16.0).color(if on { Color32::WHITE } else { p.text });
                let b = egui::Button::new(text).min_size(vec2(180.0, 36.0)).fill(if on { p.blue } else { Color32::TRANSPARENT }).corner_radius(8.0);
                if ui.add(b).clicked() {
                    v.section = s;
                    v.draft = None;
                }
            }
            ui.add_space(12.0);
            ui.label(RichText::new("Все суммы в рублях. Документ меняет учёт, только когда он проведён.").size(11.0).color(p.muted));
        },
    );
    if !v.book.problems.is_empty() || v.message.is_some() {
        egui::Panel::bottom("ledger_msgs").frame(egui::Frame::NONE.fill(p.bg).inner_margin(egui::Margin::symmetric(14, 6))).show(ui, |ui| {
            if let Some((ok, m)) = &v.message {
                ui.label(RichText::new(if *ok { format!("✔ {m}") } else { format!("✖ {m}") }).color(if *ok { p.green_dark } else { p.red_dark }));
            }
            for (id, m) in v.book.problems.iter().take(4) {
                let num = v.book.doc(*id).map(|d| d.number.clone()).unwrap_or_default();
                ui.label(RichText::new(format!("⚠ {num}: {m}")).color(p.orange));
            }
        });
    }
    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(p.bg).inner_margin(egui::Margin::same(16))).show(ui, |ui| {
        if v.draft.is_some() {
            form(v, ui, p);
        } else {
            match v.section {
                Section::Reports => reports(v, ui, p),
                Section::Catalogs => catalogs(v, ui, p),
                _ => journal(v, ui, p),
            }
        }
    });
    out.back = back;
    out
}

fn practice_bar(v: &mut LedgerView, ui: &mut Ui, p: &Palette, out: &mut Outcome) {
    let mut check = false;
    let mut skip = false;
    let mut stop = false;
    egui::Panel::top("ledger_practice").frame(egui::Frame::NONE.fill(p.green_soft).inner_margin(egui::Margin::symmetric(16, 10))).show(ui, |ui| {
        let Some(pr) = v.practice.as_mut() else { return };
        let Some(task) = pr.tasks.get(pr.index) else { return };
        let secs = pr.started.elapsed().as_secs_f32();
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("🎯 Практикум · {} / {}", pr.index + 1, pr.tasks.len())).strong().color(p.green_dark));
            ui.label(RichText::new(&task.title).size(17.0).strong().color(p.text));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.small_button("✕ Завершить").clicked() {
                    stop = true;
                }
                let goal = task.goal_seconds as f32;
                let c = if secs <= goal * 0.4 {
                    p.gold
                } else if secs <= goal * 0.7 {
                    p.green_dark
                } else if secs <= goal {
                    p.orange
                } else {
                    p.red_dark
                };
                ui.label(RichText::new(format!("⏱ {:.0} с  (норма {} с)", secs, task.goal_seconds)).strong().color(c));
            });
        });
        ui.label(RichText::new(&task.text).size(15.0).color(p.text));
        ui.horizontal(|ui| {
            if task.asks_number() {
                ui.label("Ответ:");
                let r = ui.add(egui::TextEdit::singleline(&mut pr.answer).desired_width(160.0).hint_text("0,00"));
                if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    check = true;
                }
            }
            if big_button(ui, "✔ ПРОВЕРИТЬ", p.green, vec2(150.0, 34.0)).clicked() {
                check = true;
            }
            if ui.button(if pr.show_hint { "Скрыть подсказку" } else { "💡 Подсказка" }).clicked() {
                pr.show_hint = !pr.show_hint;
            }
            if ui.button("Пропустить").clicked() {
                skip = true;
            }
            if let Some(e) = &pr.error {
                ui.label(RichText::new(format!("✖ {e}")).color(p.red_dark));
            }
        });
        if pr.show_hint {
            ui.label(RichText::new(&task.hint).size(13.0).color(p.blue_dark));
        }
    });
    let Some(pr) = v.practice.as_mut() else { return };
    let Some(task) = pr.tasks.get(pr.index) else { return };
    if check {
        let answer = Money::parse(&pr.answer);
        match task.check(&v.book, answer) {
            Ok(()) => {
                let secs = pr.started.elapsed().as_secs_f32();
                out.solved = Some((secs, task.goal_seconds));
                pr.results.push(Some(secs));
                pr.error = None;
                skip = true;
            }
            Err(e) => {
                pr.error = Some(e);
                out.failed = true;
            }
        }
    }
    if skip {
        if pr.results.len() <= pr.index {
            pr.results.push(None);
        }
        pr.index += 1;
        pr.started = crate::instant();
        pr.answer.clear();
        pr.show_hint = false;
        if !out.failed {
            pr.error = None;
        }
        if pr.index >= pr.tasks.len() {
            stop = true;
        }
    }
    if stop {
        let solved = pr.results.iter().filter(|r| r.is_some()).count();
        let total = pr.tasks.len();
        let secs = pr.began.elapsed().as_secs_f32();
        out.finished = Some((solved, total));
        v.message = Some((true, format!("Практикум завершён: {solved} из {total} за {:.0} мин {:.0} с", (secs / 60.0).floor(), secs % 60.0)));
        v.practice = None;
    }
}

fn journal(v: &mut LedgerView, ui: &mut Ui, p: &Palette) {
    let kinds = v.section.kinds();
    ui.label(RichText::new(v.section.title()).size(22.0).strong().color(p.text));
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Создать:").color(p.muted));
        for k in kinds {
            if *k == DocKind::Opening && v.section != Section::All {
                continue;
            }
            if ui.button(format!("+ {}", k.title())).clicked() {
                v.create(*k);
            }
        }
    });
    ui.add_space(6.0);
    let (from, to) = v.period();
    let mut docs: Vec<&Document> = v.book.docs.iter().filter(|d| kinds.contains(&d.body.kind()) && d.date >= from && d.date <= to).collect();
    docs.sort_by_key(|d| (d.date, d.body.kind().order(), d.id));
    let rows: Vec<(u32, bool, String, String, String, String, Money)> = docs
        .iter()
        .map(|d| (d.id, d.posted, d.date.to_string(), d.number.clone(), d.body.kind().title().to_string(), v.party(d), d.body.total()))
        .collect();
    let mut open = None;
    let mut act: Option<(&str, u32)> = None;
    ui.horizontal(|ui| {
        let sel = v.selected.filter(|s| rows.iter().any(|r| r.0 == *s));
        ui.add_enabled_ui(sel.is_some(), |ui| {
            if let Some(id) = sel {
                if ui.button("Открыть").clicked() {
                    open = Some(id);
                }
                if ui.button("✔ Провести").clicked() {
                    act = Some(("post", id));
                }
                if ui.button("Отменить проведение").clicked() {
                    act = Some(("unpost", id));
                }
                if ui.button("Дт/Кт проводки").clicked() {
                    v.show_entries = !v.show_entries;
                }
                if ui.button("🗑 Удалить").clicked() {
                    act = Some(("remove", id));
                }
            } else {
                let _ = ui.button("Открыть");
                let _ = ui.button("✔ Провести");
            }
        });
    });
    ui.add_space(4.0);
    let entries_h = if v.show_entries && v.selected.is_some() { 220.0 } else { 0.0 };
    egui::ScrollArea::vertical()
        .id_salt("journal")
        .max_height((ui.available_height() - entries_h - 10.0).max(120.0))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("journal_grid").striped(true).min_col_width(60.0).spacing(vec2(16.0, 6.0)).show(ui, |ui| {
                for h in ["", "Дата", "Номер", "Документ", "Контрагент / кому", "Сумма"] {
                    ui.label(RichText::new(h).strong().color(p.muted));
                }
                ui.end_row();
                for (id, posted, date, number, kind, party, total) in &rows {
                    let on = v.selected == Some(*id);
                    ui.label(RichText::new(if *posted { "✔" } else { "○" }).color(if *posted { p.green } else { p.muted }))
                        .on_hover_text(if *posted { "Проведён" } else { "Не проведён" });
                    let r = ui.selectable_label(on, date);
                    let r2 = ui.selectable_label(on, number);
                    let r3 = ui.selectable_label(on, kind);
                    let r4 = ui.selectable_label(on, party);
                    let r5 = ui.selectable_label(on, total.to_string());
                    for resp in [r, r2, r3, r4, r5] {
                        if resp.clicked() {
                            v.selected = Some(*id);
                        }
                        if resp.double_clicked() {
                            open = Some(*id);
                        }
                    }
                    ui.end_row();
                }
            });
            if rows.is_empty() {
                ui.label(RichText::new("За выбранный период документов нет. Создай первый кнопкой выше.").color(p.muted));
            }
        });
    if v.show_entries
        && let Some(id) = v.selected
    {
        ui.separator();
        entries_table(v, ui, p, id);
    }
    if let Some(id) = open
        && let Some(d) = v.book.doc(id).cloned()
    {
        v.draft = Some((d, false));
    }
    if let Some((what, id)) = act {
        let r = match what {
            "post" => v.book.post(id).map(|_| "проведён"),
            "unpost" => v.book.unpost(id).map(|_| "проведение отменено"),
            _ => v.book.remove(id).map(|_| "удалён"),
        };
        let num = v.book.doc(id).map(|d| d.number.clone()).unwrap_or_default();
        v.message = Some(match r {
            Ok(m) => (true, format!("{num} {m}").trim().to_string()),
            Err(e) => (false, e.to_string()),
        });
        v.save();
    }
}

fn sub_text(v: &LedgerView, s: Option<Sub>) -> String {
    s.map(|s| v.book.catalogs.sub_name(s)).unwrap_or_default()
}

fn entries_table(v: &LedgerView, ui: &mut Ui, p: &Palette, id: u32) {
    let label = v.book.doc(id).map(|d| d.label()).unwrap_or_default();
    ui.label(RichText::new(format!("Проводки: {label}")).strong().color(p.text));
    let entries: Vec<_> = v.book.entries_of(id).collect();
    if entries.is_empty() {
        ui.label(RichText::new("Проводок нет: документ не проведён.").color(p.muted));
        return;
    }
    egui::ScrollArea::vertical().id_salt(("entries", id)).max_height(200.0).show(ui, |ui| {
        egui::Grid::new(("entries_grid", id)).striped(true).spacing(vec2(16.0, 4.0)).show(ui, |ui| {
            for h in ["Дт", "Аналитика Дт", "Кт", "Аналитика Кт", "Сумма", "Кол-во", "Содержание"] {
                ui.label(RichText::new(h).strong().color(p.muted));
            }
            ui.end_row();
            for e in entries {
                ui.label(RichText::new(e.dt).strong().font(FontId::monospace(14.0)).color(p.blue_dark));
                ui.label(sub_text(v, e.dt_sub));
                ui.label(RichText::new(e.kt).strong().font(FontId::monospace(14.0)).color(p.red_dark));
                ui.label(sub_text(v, e.kt_sub));
                ui.label(RichText::new(e.amount.to_string()).monospace());
                ui.label(if e.qty != 0 { e.qty.to_string() } else { String::new() });
                ui.label(&e.memo);
                ui.end_row();
            }
        });
    });
}

fn money_edit(ui: &mut Ui, m: &mut Money) {
    let mut r = m.0 as f64 / 100.0;
    if ui.add(egui::DragValue::new(&mut r).speed(10.0).range(0.0..=1e12).max_decimals(2).suffix(" ₽")).changed() {
        *m = Money((r * 100.0).round() as i64);
    }
}

fn pick_counterparty(ui: &mut Ui, v: &Book, id: &mut u32, salt: &str) {
    let name = v.catalogs.counterparty(*id).map(|c| c.name.clone()).unwrap_or_else(|| "— выбрать —".into());
    egui::ComboBox::from_id_salt(salt).width(260.0).selected_text(name).show_ui(ui, |ui| {
        for c in &v.catalogs.counterparties {
            ui.selectable_value(id, c.id, &c.name);
        }
    });
}

fn pick_employee(ui: &mut Ui, v: &Book, id: &mut u32, salt: &str) {
    let name = v.catalogs.employee(*id).map(|c| c.name.clone()).unwrap_or_else(|| "— выбрать —".into());
    egui::ComboBox::from_id_salt(salt).width(260.0).selected_text(name).show_ui(ui, |ui| {
        for c in &v.catalogs.employees {
            ui.selectable_value(id, c.id, &c.name);
        }
    });
}

fn pick_item(ui: &mut Ui, v: &Book, id: &mut u32, salt: impl std::hash::Hash + std::fmt::Debug) {
    let name = v.catalogs.item(*id).map(|c| c.name.clone()).unwrap_or_else(|| "— выбрать —".into());
    egui::ComboBox::from_id_salt(salt).width(240.0).selected_text(name).show_ui(ui, |ui| {
        for c in &v.catalogs.items {
            ui.selectable_value(id, c.id, format!("{} ({})", c.name, c.vat.label()));
        }
    });
}

fn pick_account(ui: &mut Ui, acc: &mut String, choices: &[&str], salt: impl std::hash::Hash + std::fmt::Debug) {
    let label = |c: &str| accounts::account(c).map(|a| format!("{} {}", a.code, a.name)).unwrap_or_else(|| c.to_string());
    egui::ComboBox::from_id_salt(salt).width(240.0).selected_text(label(acc)).show_ui(ui, |ui| {
        for c in choices {
            ui.selectable_value(acc, (*c).to_string(), label(c));
        }
    });
}

fn pick_vat(ui: &mut Ui, vat: &mut Vat, salt: impl std::hash::Hash + std::fmt::Debug) {
    egui::ComboBox::from_id_salt(salt).width(90.0).selected_text(vat.label()).show_ui(ui, |ui| {
        for r in Vat::ALL {
            ui.selectable_value(vat, r, r.label());
        }
    });
}

fn goods_lines(ui: &mut Ui, book: &Book, lines: &mut Vec<GoodsLine>, p: &Palette, with_amount: bool) {
    let mut remove = None;
    egui::Grid::new("goods").striped(true).spacing(vec2(12.0, 6.0)).show(ui, |ui| {
        for h in ["№", "Номенклатура", "Количество", if with_amount { "Сумма с НДС" } else { "" }, if with_amount { "в т.ч. НДС" } else { "" }, ""]
        {
            ui.label(RichText::new(h).strong().color(p.muted));
        }
        ui.end_row();
        for (i, l) in lines.iter_mut().enumerate() {
            ui.label((i + 1).to_string());
            pick_item(ui, book, &mut l.item, ("goods_item", i));
            ui.add(egui::DragValue::new(&mut l.qty).range(0..=1_000_000));
            if with_amount {
                money_edit(ui, &mut l.amount);
                let vat = book.catalogs.item(l.item).map(|it| it.vat.inside(l.amount)).unwrap_or_default();
                ui.label(RichText::new(vat.to_string()).color(p.muted));
            } else {
                ui.label("");
                ui.label("");
            }
            if ui.small_button("✕").clicked() {
                remove = Some(i);
            }
            ui.end_row();
        }
    });
    if let Some(i) = remove {
        lines.remove(i);
    }
    if ui.button("+ Добавить строку").clicked() {
        let item = book.catalogs.items.first().map(|x| x.id).unwrap_or(0);
        lines.push(GoodsLine { item, qty: 1, amount: Money::ZERO });
    }
}

const COST_ACCOUNTS: [&str; 4] = ["26", "44", "20", "91.02"];

fn expense_lines(ui: &mut Ui, lines: &mut Vec<ExpenseLine>, p: &Palette) {
    let mut remove = None;
    egui::Grid::new("expenses").striped(true).spacing(vec2(12.0, 6.0)).show(ui, |ui| {
        for h in ["№", "Содержание", "Сумма", "НДС", "Счёт затрат", ""] {
            ui.label(RichText::new(h).strong().color(p.muted));
        }
        ui.end_row();
        for (i, l) in lines.iter_mut().enumerate() {
            ui.label((i + 1).to_string());
            ui.add(egui::TextEdit::singleline(&mut l.text).desired_width(220.0));
            money_edit(ui, &mut l.amount);
            pick_vat(ui, &mut l.vat, ("exp_vat", i));
            pick_account(ui, &mut l.account, &COST_ACCOUNTS, ("exp_acc", i));
            if ui.small_button("✕").clicked() {
                remove = Some(i);
            }
            ui.end_row();
        }
    });
    if let Some(i) = remove {
        lines.remove(i);
    }
    if ui.button("+ Добавить строку").clicked() {
        lines.push(ExpenseLine { text: String::new(), amount: Money::ZERO, vat: Vat::R22, account: "26".into() });
    }
}

fn form(v: &mut LedgerView, ui: &mut Ui, p: &Palette) {
    let Some((mut d, fresh)) = v.draft.clone() else { return };
    let book = v.book.clone();
    let mut action = 0;
    egui::ScrollArea::vertical().id_salt("form").auto_shrink([false, false]).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(d.body.kind().title()).size(22.0).strong().color(p.text));
            ui.label(RichText::new(&d.number).size(18.0).color(p.muted));
            let (t, c) = if book.doc(d.id).is_some_and(|x| x.posted) {
                ("✔ Проведён", p.green_dark)
            } else {
                ("○ Не проведён", p.muted)
            };
            ui.label(RichText::new(t).color(c));
        });
        ui.add_space(8.0);
        egui::Grid::new("form_head").spacing(vec2(12.0, 8.0)).show(ui, |ui| {
            ui.label("Дата:");
            let mut s = d.date.to_string();
            let r = ui.add(egui::TextEdit::singleline(&mut s).desired_width(110.0));
            if r.changed()
                && let Some(nd) = Date::parse(&s)
            {
                d.date = nd;
            }
            ui.end_row();
            match &mut d.body {
                Body::Receipt { supplier, account, .. } => {
                    ui.label("Поставщик:");
                    pick_counterparty(ui, &book, supplier, "f_sup");
                    ui.end_row();
                    ui.label("Приходовать на счёт:");
                    pick_account(ui, account, &["41", "10"], "f_acc41");
                    ui.end_row();
                }
                Body::Services { supplier, .. } => {
                    ui.label("Поставщик:");
                    pick_counterparty(ui, &book, supplier, "f_sup");
                    ui.end_row();
                }
                Body::Sale { customer, .. } => {
                    ui.label("Покупатель:");
                    pick_counterparty(ui, &book, customer, "f_cus");
                    ui.end_row();
                }
                Body::AssetPurchase { supplier, asset, amount, vat } => {
                    ui.label("Поставщик:");
                    pick_counterparty(ui, &book, supplier, "f_sup");
                    ui.end_row();
                    ui.label("Основное средство:");
                    let name = book.catalogs.asset(*asset).map(|a| a.name.clone()).unwrap_or_else(|| "— выбрать —".into());
                    egui::ComboBox::from_id_salt("f_asset").width(260.0).selected_text(name).show_ui(ui, |ui| {
                        for a in &book.catalogs.assets {
                            ui.selectable_value(asset, a.id, format!("{} ({} мес.)", a.name, a.life_months));
                        }
                    });
                    ui.end_row();
                    ui.label("Стоимость с НДС:");
                    money_edit(ui, amount);
                    ui.end_row();
                    ui.label("НДС:");
                    pick_vat(ui, vat, "f_vat");
                    ui.end_row();
                }
                Body::WriteOff { account, .. } => {
                    ui.label("Списать на счёт:");
                    pick_account(ui, account, &COST_ACCOUNTS, "f_wo");
                    ui.end_row();
                }
                Body::BankIn { from, amount } => {
                    ui.label("Вид операции:");
                    let mut kind = match from {
                        document::BankIn::Customer(_) => 0,
                        document::BankIn::Loan(_) => 1,
                        document::BankIn::FromCash => 2,
                    };
                    let names = ["Оплата от покупателя", "Получение займа", "Взнос наличных из кассы"];
                    egui::ComboBox::from_id_salt("f_bin").width(260.0).selected_text(names.get(kind).copied().unwrap_or("")).show_ui(ui, |ui| {
                        for (i, n) in names.iter().enumerate() {
                            ui.selectable_value(&mut kind, i, *n);
                        }
                    });
                    ui.end_row();
                    let mut cp = match from {
                        document::BankIn::Customer(c) | document::BankIn::Loan(c) => *c,
                        document::BankIn::FromCash => book.catalogs.counterparties.first().map(|c| c.id).unwrap_or(0),
                    };
                    if kind < 2 {
                        ui.label("Плательщик:");
                        pick_counterparty(ui, &book, &mut cp, "f_payer");
                        ui.end_row();
                    }
                    *from = match kind {
                        0 => document::BankIn::Customer(cp),
                        1 => document::BankIn::Loan(cp),
                        _ => document::BankIn::FromCash,
                    };
                    ui.label("Сумма:");
                    money_edit(ui, amount);
                    ui.end_row();
                }
                Body::BankOut { to, amount } => {
                    let names = [
                        "Оплата поставщику",
                        "Уплата налога",
                        "Уплата страховых взносов",
                        "Выплата зарплаты",
                        "Комиссия банка",
                        "Возврат займа",
                        "Снятие наличных в кассу",
                    ];
                    let mut kind = match to {
                        document::BankOut::Supplier(_) => 0,
                        document::BankOut::Tax(_) => 1,
                        document::BankOut::Insurance => 2,
                        document::BankOut::Salary(_) => 3,
                        document::BankOut::Fee => 4,
                        document::BankOut::LoanRepay(_) => 5,
                        document::BankOut::ToCash => 6,
                    };
                    ui.label("Вид операции:");
                    egui::ComboBox::from_id_salt("f_bout").width(260.0).selected_text(names.get(kind).copied().unwrap_or("")).show_ui(ui, |ui| {
                        for (i, n) in names.iter().enumerate() {
                            ui.selectable_value(&mut kind, i, *n);
                        }
                    });
                    ui.end_row();
                    let mut cp = match to {
                        document::BankOut::Supplier(c) | document::BankOut::LoanRepay(c) => *c,
                        _ => book.catalogs.counterparties.first().map(|c| c.id).unwrap_or(0),
                    };
                    let mut emp = match to {
                        document::BankOut::Salary(e) => *e,
                        _ => book.catalogs.employees.first().map(|c| c.id).unwrap_or(0),
                    };
                    let mut tax = match to {
                        document::BankOut::Tax(a) => a.clone(),
                        _ => "68.02".into(),
                    };
                    match kind {
                        0 | 5 => {
                            ui.label("Получатель:");
                            pick_counterparty(ui, &book, &mut cp, "f_rcpt");
                            ui.end_row();
                        }
                        1 => {
                            ui.label("Налог:");
                            pick_account(ui, &mut tax, &["68.01", "68.02", "68.04"], "f_tax");
                            ui.end_row();
                        }
                        3 => {
                            ui.label("Сотрудник:");
                            pick_employee(ui, &book, &mut emp, "f_emp");
                            ui.end_row();
                        }
                        _ => {}
                    }
                    *to = match kind {
                        0 => document::BankOut::Supplier(cp),
                        1 => document::BankOut::Tax(tax),
                        2 => document::BankOut::Insurance,
                        3 => document::BankOut::Salary(emp),
                        4 => document::BankOut::Fee,
                        5 => document::BankOut::LoanRepay(cp),
                        _ => document::BankOut::ToCash,
                    };
                    ui.label("Сумма:");
                    money_edit(ui, amount);
                    ui.end_row();
                }
                Body::CashIn { from, amount } => {
                    let names = ["Оплата от покупателя", "Возврат подотчётных денег"];
                    let mut kind = usize::from(matches!(from, document::CashIn::AdvanceReturn(_)));
                    ui.label("Вид операции:");
                    egui::ComboBox::from_id_salt("f_cin").width(260.0).selected_text(names.get(kind).copied().unwrap_or("")).show_ui(ui, |ui| {
                        for (i, n) in names.iter().enumerate() {
                            ui.selectable_value(&mut kind, i, *n);
                        }
                    });
                    ui.end_row();
                    let (mut cp, mut emp) = match from {
                        document::CashIn::Customer(c) => (*c, book.catalogs.employees.first().map(|e| e.id).unwrap_or(0)),
                        document::CashIn::AdvanceReturn(e) => (book.catalogs.counterparties.first().map(|c| c.id).unwrap_or(0), *e),
                    };
                    if kind == 0 {
                        ui.label("Покупатель:");
                        pick_counterparty(ui, &book, &mut cp, "f_cincp");
                    } else {
                        ui.label("Сотрудник:");
                        pick_employee(ui, &book, &mut emp, "f_cinemp");
                    }
                    ui.end_row();
                    *from = if kind == 0 { document::CashIn::Customer(cp) } else { document::CashIn::AdvanceReturn(emp) };
                    ui.label("Сумма:");
                    money_edit(ui, amount);
                    ui.end_row();
                }
                Body::CashOut { to, amount } => {
                    let names = ["Выдача под отчёт", "Выплата зарплаты", "Оплата поставщику"];
                    let mut kind = match to {
                        document::CashOut::Advance(_) => 0,
                        document::CashOut::Salary(_) => 1,
                        document::CashOut::Supplier(_) => 2,
                    };
                    ui.label("Вид операции:");
                    egui::ComboBox::from_id_salt("f_cout").width(260.0).selected_text(names.get(kind).copied().unwrap_or("")).show_ui(ui, |ui| {
                        for (i, n) in names.iter().enumerate() {
                            ui.selectable_value(&mut kind, i, *n);
                        }
                    });
                    ui.end_row();
                    let (mut cp, mut emp) = match to {
                        document::CashOut::Supplier(c) => (*c, book.catalogs.employees.first().map(|e| e.id).unwrap_or(0)),
                        document::CashOut::Advance(e) | document::CashOut::Salary(e) => {
                            (book.catalogs.counterparties.first().map(|c| c.id).unwrap_or(0), *e)
                        }
                    };
                    if kind == 2 {
                        ui.label("Поставщик:");
                        pick_counterparty(ui, &book, &mut cp, "f_coutcp");
                    } else {
                        ui.label("Сотрудник:");
                        pick_employee(ui, &book, &mut emp, "f_coutemp");
                    }
                    ui.end_row();
                    *to = match kind {
                        0 => document::CashOut::Advance(emp),
                        1 => document::CashOut::Salary(emp),
                        _ => document::CashOut::Supplier(cp),
                    };
                    ui.label("Сумма:");
                    money_edit(ui, amount);
                    ui.end_row();
                }
                Body::AdvanceReport { employee, .. } => {
                    ui.label("Подотчётное лицо:");
                    pick_employee(ui, &book, employee, "f_adv");
                    ui.end_row();
                }
                Body::Payroll { account, .. } => {
                    ui.label("Счёт затрат:");
                    pick_account(ui, account, &["26", "44", "20"], "f_pay");
                    ui.end_row();
                }
                Body::Opening { .. } | Body::MonthClose => {}
            }
        });
        ui.add_space(10.0);
        match &mut d.body {
            Body::Receipt { lines, .. } | Body::Sale { lines, .. } => goods_lines(ui, &book, lines, p, true),
            Body::WriteOff { lines, .. } => goods_lines(ui, &book, lines, p, false),
            Body::Services { lines, .. } | Body::AdvanceReport { lines, .. } => expense_lines(ui, lines, p),
            Body::Payroll { lines, .. } => {
                let mut remove = None;
                egui::Grid::new("payroll").striped(true).spacing(vec2(12.0, 6.0)).show(ui, |ui| {
                    for h in ["Сотрудник", "Начислено", "НДФЛ 13%", "Взносы 30%", "К выплате", ""] {
                        ui.label(RichText::new(h).strong().color(p.muted));
                    }
                    ui.end_row();
                    for (i, (emp, gross)) in lines.iter_mut().enumerate() {
                        pick_employee(ui, &book, emp, &format!("pay_emp_{i}"));
                        money_edit(ui, gross);
                        let ndfl = gross.mul_div(gridcraft_ledger::NDFL_PERCENT, 100);
                        ui.label(ndfl.to_string());
                        ui.label(gross.mul_div(gridcraft_ledger::CONTRIBUTIONS_PERCENT, 100).to_string());
                        ui.label(RichText::new((*gross - ndfl).to_string()).strong());
                        if ui.small_button("✕").clicked() {
                            remove = Some(i);
                        }
                        ui.end_row();
                    }
                });
                if let Some(i) = remove {
                    lines.remove(i);
                }
                if ui.button("+ Сотрудник").clicked() {
                    let e = book.catalogs.employees.first();
                    lines.push((e.map(|e| e.id).unwrap_or(0), e.map(|e| e.salary).unwrap_or_default()));
                }
            }
            Body::Opening { lines } => {
                let mut remove = None;
                let codes: Vec<&str> = accounts::postable().filter(|a| a.code != "000").map(|a| a.code).collect();
                egui::Grid::new("opening").striped(true).spacing(vec2(12.0, 6.0)).show(ui, |ui| {
                    for h in ["Счёт", "Дебет", "Кредит", "Кол-во", ""] {
                        ui.label(RichText::new(h).strong().color(p.muted));
                    }
                    ui.end_row();
                    for (i, l) in lines.iter_mut().enumerate() {
                        pick_account(ui, &mut l.account, &codes, ("open_acc", i));
                        money_edit(ui, &mut l.debit);
                        money_edit(ui, &mut l.credit);
                        ui.add(egui::DragValue::new(&mut l.qty));
                        if ui.small_button("✕").clicked() {
                            remove = Some(i);
                        }
                        ui.end_row();
                    }
                });
                if let Some(i) = remove {
                    lines.remove(i);
                }
                if ui.button("+ Строка").clicked() {
                    lines.push(OpeningLine { account: "51".into(), sub: None, debit: Money::ZERO, credit: Money::ZERO, qty: 0 });
                }
            }
            Body::MonthClose => {
                ui.label(RichText::new(format!("Закрытие месяца: {} {}", month_name(d.date.m), d.date.y)).size(16.0).color(p.text));
                for step in [
                    "1. Амортизация основных средств (Дт 26 Кт 02)",
                    "2. Вычет входящего НДС (Дт 68.02 Кт 19)",
                    "3. Закрытие затрат: 26 → 90.08, 44 → 90.07, 20 → 90.02",
                    "4. Финансовый результат: 90.09 и 91.09 → 99",
                    "5. Налог на прибыль 25% (Дт 99 Кт 68.04), если месяц прибыльный",
                ] {
                    ui.label(RichText::new(step).color(p.muted));
                }
                ui.label(RichText::new("Дата документа — последний день месяца.").size(12.0).color(p.muted));
            }
            _ => {}
        }
        ui.add_space(6.0);
        let total = d.body.total();
        if !total.is_zero() {
            ui.label(RichText::new(format!("Итого: {total} ₽")).size(18.0).strong().color(p.text));
        }
        ui.horizontal(|ui| {
            ui.label("Комментарий:");
            ui.add(egui::TextEdit::singleline(&mut d.comment).desired_width(400.0));
        });
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if big_button(ui, "Провести и закрыть", p.green, vec2(220.0, 42.0)).clicked() {
                action = 1;
            }
            if ghost_button(ui, "Записать", p, vec2(140.0, 42.0)).clicked() {
                action = 2;
            }
            if ghost_button(ui, "Провести", p, vec2(140.0, 42.0)).clicked() {
                action = 3;
            }
            if ghost_button(ui, "Закрыть", p, vec2(140.0, 42.0)).clicked() {
                action = 4;
            }
        });
        ui.add_space(12.0);
        entries_table(v, ui, p, d.id);
    });
    v.draft = Some((d, fresh));
    match action {
        1 => {
            if v.write(true) {
                v.draft = None;
            }
        }
        2 => {
            v.write(false);
        }
        3 => {
            v.write(true);
        }
        4 => v.close_form(),
        _ => {}
    }
}

fn money_cell(ui: &mut Ui, m: Money) {
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.label(RichText::new(if m.is_zero() { String::new() } else { m.to_string() }).monospace());
    });
}

fn reports(v: &mut LedgerView, ui: &mut Ui, p: &Palette) {
    ui.horizontal_wrapped(|ui| {
        for r in ReportKind::ALL {
            if ui.selectable_label(v.report == r, r.title()).clicked() {
                v.report = r;
            }
        }
    });
    let (from, to) = v.period();
    ui.label(RichText::new(format!("{} · {} — {}", v.report.title(), from, to)).size(20.0).strong().color(p.text));
    ui.add_space(6.0);
    let codes: Vec<&str> = accounts::CHART.iter().filter(|a| a.code != "000").map(|a| a.code).collect();
    if matches!(v.report, ReportKind::Card | ReportKind::Analysis) {
        ui.horizontal(|ui| {
            ui.label("Счёт:");
            pick_account(ui, &mut v.account, &codes, "rep_acc");
        });
    }
    let mut jump: Option<String> = None;
    egui::ScrollArea::both().id_salt("report").auto_shrink([false, false]).show(ui, |ui| match v.report {
        ReportKind::Trial => {
            let rows = report::trial_balance(&v.book, from, to);
            egui::Grid::new("tb").striped(true).min_col_width(60.0).spacing(vec2(14.0, 5.0)).show(ui, |ui| {
                for h in ["Счёт", "Наименование", "Нач. Дт", "Нач. Кт", "Оборот Дт", "Оборот Кт", "Кон. Дт", "Кон. Кт"]
                {
                    ui.label(RichText::new(h).strong().color(p.muted));
                }
                ui.end_row();
                for r in &rows {
                    let sub = r.code.contains('.');
                    let total = r.code.is_empty();
                    if total {
                        ui.label("");
                        ui.label(RichText::new(&r.name).strong());
                    } else {
                        let link = ui
                            .link(RichText::new(if sub { format!("  {}", r.code) } else { r.code.clone() }).monospace())
                            .on_hover_text("Открыть карточку счёта");
                        if link.clicked() {
                            jump = Some(r.code.clone());
                        }
                        let name: String =
                            if r.name.chars().count() > 34 { r.name.chars().take(32).chain("…".chars()).collect() } else { r.name.clone() };
                        ui.label(RichText::new(name).color(if sub { p.muted } else { p.text })).on_hover_text(&r.name);
                    }
                    for m in [r.open_dt, r.open_kt, r.turn_dt, r.turn_kt, r.close_dt, r.close_kt] {
                        money_cell(ui, m);
                    }
                    ui.end_row();
                }
            });
        }
        ReportKind::Card => {
            let (open, lines) = report::account_card(&v.book, &v.account, None, from, to);
            ui.label(format!("Сальдо на начало: {}", side(open)));
            egui::Grid::new("card").striped(true).spacing(vec2(14.0, 5.0)).show(ui, |ui| {
                for h in ["Дата", "Документ", "Операция", "Корр. счёт", "Аналитика", "Дебет", "Кредит", "Сальдо"]
                {
                    ui.label(RichText::new(h).strong().color(p.muted));
                }
                ui.end_row();
                for l in &lines {
                    ui.label(l.date.to_string());
                    ui.label(&l.document);
                    ui.label(&l.memo);
                    ui.label(RichText::new(l.corr).monospace());
                    ui.label(sub_text(v, l.sub));
                    money_cell(ui, l.dt);
                    money_cell(ui, l.kt);
                    ui.label(side(l.balance));
                    ui.end_row();
                }
            });
            let close = lines.last().map(|l| l.balance).unwrap_or(open);
            ui.label(RichText::new(format!("Сальдо на конец: {}", side(close))).strong());
        }
        ReportKind::Analysis => {
            let rows = report::account_analysis(&v.book, &v.account, from, to);
            egui::Grid::new("analysis").striped(true).min_col_width(120.0).spacing(vec2(14.0, 5.0)).show(ui, |ui| {
                for h in ["Корр. счёт", "Наименование", "Оборот Дт", "Оборот Кт"] {
                    ui.label(RichText::new(h).strong().color(p.muted));
                }
                ui.end_row();
                for (acc, dt, kt) in &rows {
                    ui.label(RichText::new(*acc).monospace());
                    ui.label(accounts::account(acc).map(|a| a.name).unwrap_or(""));
                    money_cell(ui, *dt);
                    money_cell(ui, *kt);
                    ui.end_row();
                }
            });
        }
        ReportKind::Stock => {
            for acc in ["41", "10"] {
                ui.label(RichText::new(format!("Счёт {acc} {}", accounts::account(acc).map(|a| a.name).unwrap_or(""))).strong());
                let rows = report::account_balance(&v.book, acc, from, to);
                egui::Grid::new(("stock", acc)).striped(true).min_col_width(110.0).spacing(vec2(14.0, 5.0)).show(ui, |ui| {
                    for h in ["Номенклатура", "Количество", "Сумма", "Средняя цена"] {
                        ui.label(RichText::new(h).strong().color(p.muted));
                    }
                    ui.end_row();
                    for r in &rows {
                        ui.label(&r.name);
                        ui.label(r.qty.to_string());
                        money_cell(ui, r.row.close_dt);
                        money_cell(ui, if r.qty > 0 { r.row.close_dt.mul_div(1, r.qty) } else { Money::ZERO });
                        ui.end_row();
                    }
                });
                ui.add_space(10.0);
            }
        }
        ReportKind::Results => {
            let r = report::results(&v.book, from, to);
            egui::Grid::new("results").striped(true).min_col_width(160.0).spacing(vec2(14.0, 6.0)).show(ui, |ui| {
                for (name, m, strong) in [
                    ("Выручка (с НДС)", r.revenue, false),
                    ("НДС с выручки", r.vat, false),
                    ("Себестоимость продаж", r.cost_of_sales, false),
                    ("Расходы на продажу", r.selling, false),
                    ("Управленческие расходы", r.admin, false),
                    ("Прибыль (убыток) от продаж", r.sales_profit, true),
                    ("Прочие доходы", r.other_income, false),
                    ("Прочие расходы", r.other_expense, false),
                    ("Прибыль до налогообложения", r.before_tax, true),
                    ("Налог на прибыль", r.tax, false),
                    ("Чистая прибыль", r.net, true),
                ] {
                    let t = RichText::new(name);
                    ui.label(if strong { t.strong() } else { t });
                    let mt = RichText::new(m.to_string()).monospace().color(if m < Money::ZERO { p.red_dark } else { p.text });
                    ui.label(if strong { mt.strong() } else { mt });
                    ui.end_row();
                }
            });
        }
    });
    if let Some(code) = jump {
        v.account = code;
        v.report = ReportKind::Card;
    }
}

fn side(m: Money) -> String {
    if m > Money::ZERO {
        format!("Дт {m}")
    } else if m < Money::ZERO {
        format!("Кт {}", -m)
    } else {
        "0".into()
    }
}

fn catalogs(v: &mut LedgerView, ui: &mut Ui, p: &Palette) {
    ui.horizontal(|ui| {
        for (k, name) in [
            (CatalogKind::Counterparties, "Контрагенты"),
            (CatalogKind::Items, "Номенклатура"),
            (CatalogKind::Employees, "Сотрудники"),
            (CatalogKind::Assets, "Основные средства"),
        ] {
            if ui.selectable_label(v.catalog == k, name).clicked() {
                v.catalog = k;
            }
        }
    });
    ui.add_space(8.0);
    let c = &v.book.catalogs;
    egui::ScrollArea::vertical().id_salt("catalog").max_height(ui.available_height() - 80.0).auto_shrink([false, false]).show(ui, |ui| {
        egui::Grid::new(("cat", v.catalog as u8)).striped(true).min_col_width(140.0).spacing(vec2(14.0, 5.0)).show(ui, |ui| match v.catalog {
            CatalogKind::Counterparties => {
                ui.label(RichText::new("Наименование").strong().color(p.muted));
                ui.label(RichText::new("ИНН").strong().color(p.muted));
                ui.label(RichText::new("Сальдо 60").strong().color(p.muted));
                ui.label(RichText::new("Сальдо 62").strong().color(p.muted));
                ui.end_row();
                let (_, to) = v.period();
                for x in &c.counterparties {
                    ui.label(&x.name);
                    ui.label(&x.inn);
                    let bal = |acc: &str| {
                        report::account_balance(&v.book, acc, to, to)
                            .iter()
                            .find(|r| r.sub == Some(Sub::Counterparty(x.id)))
                            .map(|r| r.row.close_dt - r.row.close_kt)
                            .unwrap_or_default()
                    };
                    ui.label(side(bal("60")));
                    ui.label(side(bal("62")));
                    ui.end_row();
                }
            }
            CatalogKind::Items => {
                for h in ["Наименование", "Ед.", "НДС"] {
                    ui.label(RichText::new(h).strong().color(p.muted));
                }
                ui.end_row();
                for x in &c.items {
                    ui.label(&x.name);
                    ui.label(&x.unit);
                    ui.label(x.vat.label());
                    ui.end_row();
                }
            }
            CatalogKind::Employees => {
                for h in ["ФИО", "Должность", "Оклад"] {
                    ui.label(RichText::new(h).strong().color(p.muted));
                }
                ui.end_row();
                for x in &c.employees {
                    ui.label(&x.name);
                    ui.label(&x.position);
                    ui.label(x.salary.to_string());
                    ui.end_row();
                }
            }
            CatalogKind::Assets => {
                for h in ["Наименование", "Срок, мес."] {
                    ui.label(RichText::new(h).strong().color(p.muted));
                }
                ui.end_row();
                for x in &c.assets {
                    ui.label(&x.name);
                    ui.label(x.life_months.to_string());
                    ui.end_row();
                }
            }
        });
    });
    ui.separator();
    let (hint, extra) = match v.catalog {
        CatalogKind::Counterparties => ("Название", "ИНН"),
        CatalogKind::Items => ("Название", "Ставка НДС: 22, 10, 0 или пусто"),
        CatalogKind::Employees => ("ФИО", "Оклад, ₽"),
        CatalogKind::Assets => ("Название", "Срок, месяцев"),
    };
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(&mut v.new_name).hint_text(hint).desired_width(260.0));
        ui.add(egui::TextEdit::singleline(&mut v.new_extra).hint_text(extra).desired_width(200.0));
        if ui.button("+ Добавить").clicked() && !v.new_name.trim().is_empty() {
            let name = v.new_name.trim().to_string();
            let extra = v.new_extra.trim().to_string();
            let c = &mut v.book.catalogs;
            match v.catalog {
                CatalogKind::Counterparties => {
                    c.add_counterparty(&name, &extra);
                }
                CatalogKind::Items => {
                    let vat = match extra.as_str() {
                        "10" => Vat::R10,
                        "0" => Vat::Zero,
                        "" => Vat::None,
                        _ => Vat::R22,
                    };
                    c.add_item(&name, "шт", vat);
                }
                CatalogKind::Employees => {
                    c.add_employee(&name, "", Money::parse(&extra).unwrap_or_default());
                }
                CatalogKind::Assets => {
                    c.add_asset(&name, extra.parse().unwrap_or(36));
                }
            }
            v.new_name.clear();
            v.new_extra.clear();
            v.save();
        }
    });
}
