//! Screens: path (home), lesson, case/interview, lesson complete, report, statistics.

use crate::view::{self, Mood, Palette, big_button, chip, ghost_button, mascot, palette, progress};
use crate::{LearningApp, Screen, now};
use egui::{Align, Color32, FontId, Layout, RichText, Sense, Stroke, Ui, pos2, vec2};
use gridcraft_learn_core::Profile;
use gridcraft_learn_core::quiz::Track;
use gridcraft_learn_core::{
    Mode, Slot,
    content::{CASES, LEVELS, SKILLS, UNITS},
};
use gridcraft_tr::{t as tr, ts};

fn pal(app: &LearningApp) -> Palette {
    palette(app.sheet.ui.dark)
}

fn unit_color(i: usize) -> Color32 {
    UNITS.get(i).map(|u| Color32::from_rgb(u.color[0], u.color[1], u.color[2])).unwrap_or(Color32::GRAY)
}

fn level_name(level: u8) -> &'static str {
    if level == 0 { ts("Новый") } else { ts(LEVELS.get(usize::from(level) - 1).copied().unwrap_or("")) }
}

fn page(ui: &mut Ui, p: &Palette, add: impl FnOnce(&mut Ui)) {
    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(p.bg).inner_margin(egui::Margin::same(0))).show(ui, add);
}

fn top_bar(app: &mut LearningApp, ui: &mut Ui, p: &Palette) {
    egui::Panel::top("learn_top")
        .frame(egui::Frame::NONE.fill(p.bg).inner_margin(egui::Margin::symmetric(20, 10)).stroke(Stroke::new(2.0, p.border)))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.label(RichText::new("Task").size(24.0).strong().color(p.green));
                ui.label(RichText::new("athlon").size(24.0).strong().color(p.text));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button(ts("⚙ Настройки")).clicked() {
                        app.show_settings = true;
                    }
                    if ui.button(ts("📊 Статистика")).clicked() {
                        app.screen = Screen::Stats;
                    }
                    if ui.button(ts("📒 Учёт")).on_hover_text(ts("Учётная программа: документы, проводки, ОСВ")).clicked()
                    {
                        app.screen = Screen::Ledger;
                    }
                    ui.add_space(16.0);
                    let t = now();
                    ui.label(RichText::new(format!("👑 {:.0}%", app.profile.mastery() * 100.0)).size(17.0).strong().color(p.gold))
                        .on_hover_text(ts("Освоено от всего пути"));
                    ui.add_space(10.0);
                    ui.label(RichText::new(format!("⚡ {}", app.profile.xp)).size(17.0).strong().color(p.blue))
                        .on_hover_text(ts("Опыт за всё время"));
                    ui.add_space(10.0);
                    let streak = app.profile.current_streak(t);
                    let c = if app.profile.today_xp(t) > 0 { p.orange } else { p.muted };
                    ui.label(RichText::new(format!("🔥 {streak}")).size(17.0).strong().color(c)).on_hover_text(ts("Дней подряд"));
                });
            });
            if let Some(n) = app.notice.clone() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(n).color(p.muted));
                    if ui.small_button("✕").clicked() {
                        app.notice = None;
                    }
                });
            }
        });
}

fn card(ui: &mut Ui, p: &Palette, fill: Option<Color32>, add: impl FnOnce(&mut Ui)) {
    let mut f = egui::Frame::NONE.stroke(Stroke::new(2.0, p.border)).corner_radius(16.0).inner_margin(egui::Margin::same(14));
    if let Some(c) = fill {
        f = f.fill(c);
    }
    f.show(ui, |ui| {
        ui.set_width(ui.available_width());
        add(ui);
    });
    ui.add_space(12.0);
}

fn side_column(app: &mut LearningApp, ui: &mut Ui, p: &Palette, t: u64, frontier: Option<&'static str>) {
    let first = app.profile.log.is_empty();
    card(ui, p, Some(p.card), |ui| {
        let next = frontier.and_then(|f| SKILLS.iter().find(|s| s.id == f));
        let mut line = app.greeting.clone().unwrap_or_default();
        if !first && let Some(s) = next {
            line.push_str(&tr(&format!("\nДальше: «{}».", ts(s.name))));
        }
        if !app.profile.remediation.is_empty() {
            line.push_str(ts(" Сначала разберём ошибки."));
        }
        ui.horizontal(|ui| {
            let mood = if app.profile.streak_at_risk(t) { Mood::Think } else { Mood::Happy };
            mascot(ui, 78.0, mood, p);
            view::bubble(ui, &line, p, (ui.available_width() - 30.0).max(120.0));
        });
        ui.add_space(10.0);
        let label = if first { ts("НАЧАТЬ") } else { ts("ПРОДОЛЖИТЬ") };
        if big_button(ui, label, p.green, vec2(ui.available_width(), 56.0)).clicked() {
            let r = app.start_lesson(None);
            app.handle(r);
        }
        ui.label(RichText::new(ts("или нажми Enter")).size(12.0).color(p.muted));
    });
    card(ui, p, None, |ui| {
        let today = app.profile.today_xp(t);
        ui.horizontal(|ui| {
            ui.label(RichText::new(ts("Цель на день")).size(17.0).strong().color(p.text));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new(format!("{today}/{} XP", app.profile.goal)).color(p.muted));
            });
        });
        progress(ui, today as f32 / app.profile.goal.max(1) as f32, p.gold, p.grey, vec2(ui.available_width(), 16.0));
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let streak = app.profile.current_streak(t);
            ui.label(RichText::new(tr(&format!("🔥 {streak} дн."))).strong().color(if today > 0 { p.orange } else { p.muted }));
            ui.add_space(10.0);
            ui.label(RichText::new(format!("💎 {}", app.profile.gems)).strong().color(p.blue));
            ui.add_space(10.0);
            ui.label(RichText::new(format!("🧊 {}", app.profile.freezes)).color(p.blue_dark))
                .on_hover_text(ts("Заморозки серии: спасают один пропущенный день"));
        });
        if app.profile.streak_at_risk(t) {
            ui.label(RichText::new(ts("Сегодня ещё не занимался — серия сгорит в полночь!")).size(12.0).color(p.red_dark));
        }
    });
    app.profile.refresh_quests(t);
    card(ui, p, None, |ui| {
        ui.label(RichText::new(ts("📋 Задания дня")).size(17.0).strong().color(p.text));
        for q in &app.profile.quests {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let icon = if q.done { "✅" } else { "▫" };
                ui.label(RichText::new(format!("{icon} {}", q.title())).color(if q.done { p.green_dark } else { p.text }));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(format!("+{} 💎", q.reward)).size(12.0).color(p.blue));
                });
            });
            progress(
                ui,
                q.progress as f32 / q.target.max(1) as f32,
                if q.done { p.green } else { p.orange },
                p.grey,
                vec2(ui.available_width(), 8.0),
            );
        }
    });
    card(ui, p, None, |ui| {
        ui.label(RichText::new(ts("⏱ Экспресс-тест")).size(17.0).strong().color(p.text));
        ui.label(
            RichText::new(ts(
                "15 вопросов на время, как онлайн-тесты при отборе в крупные компании: формулы, клавиши, сводные, выводы, SQL. Клавиши 1–4.",
            ))
            .size(13.0)
            .color(p.muted),
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let w = (ui.available_width() - 16.0) / 3.0;
            for (label, track, color) in
                [("EXCEL", Track::Excel, p.green), ("SQL", Track::Sql, Color32::from_rgb(70, 90, 220)), (ts("МИКС"), Track::Mixed, p.orange)]
            {
                if big_button(ui, label, color, vec2(w, 42.0)).clicked() {
                    app.start_blitz(track);
                }
            }
        });
        if let Some(b) = app.profile.blitz.last() {
            ui.label(RichText::new(tr(&format!("Последний: {}/{} верно, {} очков", b.correct, b.total, b.points))).size(12.0).color(p.muted));
        }
    });
    card(ui, p, None, |ui| {
        ui.label(RichText::new(ts("🏁 Спринт")).size(17.0).strong().color(p.text));
        ui.label(
            RichText::new(ts("10 настоящих заданий на секундомер. Ошибка — +10 с штрафа. Бей свой рекорд, как в Excel-спорте."))
                .size(13.0)
                .color(p.muted),
        );
        ui.add_space(6.0);
        let ready = !app.profile.sprint_skills().is_empty();
        if ready {
            if big_button(ui, ts("СТАРТ"), p.orange, vec2(ui.available_width(), 44.0)).clicked() {
                let r = app.start_sprint(None);
                app.handle(r);
            }
            if let Some(best) = app.profile.sprint_best("mix") {
                ui.label(RichText::new(tr(&format!("Рекорд: {}", clock(best)))).size(12.0).color(p.muted));
            }
        } else {
            ui.label(RichText::new(ts("Откроется, когда освоишь первые навыки (уровень 2).")).size(12.0).color(p.muted));
        }
    });
    card(ui, p, None, |ui| {
        ui.label(RichText::new(ts("📤 Задание в Excel")).size(17.0).strong().color(p.text));
        ui.label(
            RichText::new(ts(
                "Рабочий кейс файлом .xlsx: реши в настоящем Excel без тренажёра, сохрани — Taskathlon проверит файл теми же проверками.",
            ))
            .size(13.0)
            .color(p.muted),
        );
        ui.add_space(6.0);
        let excel = Color32::from_rgb(33, 115, 70);
        if let Some(t) = app.profile.pending_transfer.clone() {
            let name = std::path::Path::new(&t.path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            ui.label(RichText::new(format!("📄 {name}")).size(13.0).strong().color(p.text)).on_hover_text(&t.path);
            if big_button(ui, ts("ПРОВЕРИТЬ ФАЙЛ"), excel, vec2(ui.available_width(), 44.0)).clicked() {
                let r = app.check_transfer();
                app.handle(r);
            }
            ui.horizontal(|ui| {
                if ui.button(ts("Открыть файл")).clicked() {
                    app.open_file();
                }
                if ui.button(ts("Другое задание")).clicked() {
                    let r = app.start_transfer();
                    app.handle(r);
                }
            });
        } else if big_button(ui, ts("ПОЛУЧИТЬ ФАЙЛ"), excel, vec2(ui.available_width(), 44.0)).clicked() {
            let r = app.start_transfer();
            app.handle(r);
        }
        let done = app.profile.transfers.iter().filter(|t| t.score >= 0.999).count();
        if !app.profile.transfers.is_empty() {
            ui.label(RichText::new(tr(&format!("Сдано без замечаний: {done} из {}", app.profile.transfers.len()))).size(12.0).color(p.muted));
        }
    });
    card(ui, p, None, |ui| {
        ui.label(RichText::new(ts("📈 Срез навыка")).size(17.0).strong().color(p.text));
        ui.label(
            RichText::new(ts("8 задач на новых данных, без подсказок. Пройди до обучения, после и через неделю — увидишь, держится ли навык."))
                .size(13.0)
                .color(p.muted),
        );
        ui.add_space(6.0);
        match app.profile.next_checkpoint(now()) {
            Ok(phase) => {
                let label = tr(&format!("СРЕЗ: {}", ts(phase.label()).to_uppercase()));
                if big_button(ui, &label, Color32::from_rgb(20, 160, 170), vec2(ui.available_width(), 44.0)).clicked() {
                    let r = app.start_checkpoint();
                    app.handle(r);
                }
            }
            Err(why) => {
                ui.label(RichText::new(why).size(12.0).color(p.muted));
            }
        }
        let line: Vec<String> =
            app.profile.checkpoints.iter().rev().take(3).rev().map(|c| format!("{} {}/{}", ts(c.phase.label()), c.solved, c.total)).collect();
        if !line.is_empty() {
            ui.label(RichText::new(line.join(" · ")).size(12.0).color(p.muted));
        }
    });
    card(ui, p, None, |ui| {
        ui.label(RichText::new(ts("📒 Учёт")).size(17.0).strong().color(p.text));
        ui.label(
            RichText::new(ts(
                "Учётная программа как на работе: документы, «Провести», проводки, ОСВ, закрытие месяца. Демо-компания уже ведёт январь.",
            ))
            .size(13.0)
            .color(p.muted),
        );
        ui.add_space(6.0);
        if big_button(ui, ts("ОТКРЫТЬ УЧЁТ"), Color32::from_rgb(220, 90, 160), vec2(ui.available_width(), 44.0)).clicked() {
            app.screen = Screen::Ledger;
        }
    });
    card(ui, p, None, |ui| {
        ui.label(RichText::new(ts("🎯 Собеседование")).size(17.0).strong().color(p.text));
        ui.label(RichText::new(ts("Тестовое задание как у работодателя: письмо, данные, таймер, без подсказок.")).size(13.0).color(p.muted));
        ui.add_space(6.0);
        if big_button(ui, ts("ПРОЙТИ ТЕСТОВОЕ"), p.blue, vec2(ui.available_width(), 44.0)).clicked() {
            let r = app.start_interview();
            app.handle(r);
        }
        if let Some(last) = app.profile.interviews.last() {
            ui.label(
                RichText::new(tr(&format!("Последнее: {:.0}% за {} мин", last.score * 100.0, (last.seconds / 60.0).ceil())))
                    .size(12.0)
                    .color(p.muted),
            );
        }
    });
    card(ui, p, None, |ui| {
        ui.label(RichText::new(tr(&format!("💎 Лавка · у тебя {}", app.profile.gems))).size(17.0).strong().color(p.text));
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr(&format!("🧊 Заморозка серии ({}/2)", app.profile.freezes))).color(p.text));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button(format!("{} 💎", Profile::FREEZE_PRICE)).clicked() {
                    match app.profile.buy_freeze() {
                        Ok(()) => app.notice = Some(ts("🧊 Заморозка куплена: один пропуск не сломает серию.").into()),
                        Err(e) => app.notice = Some(e),
                    }
                }
            });
        });
        match app.profile.wager.clone() {
            Some(w) => {
                let left = (w.start_day + u64::from(w.days)).saturating_sub(app.profile.day(t));
                ui.label(RichText::new(tr(&format!("🎲 Ставка: ещё {left} дн. без пропусков → +{} 💎", w.stake * 2))).color(p.orange));
            }
            None => {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(ts("🎲 Ставка: 7 дней подряд")).color(p.text))
                        .on_hover_text(ts("Ставишь 50 💎. Не пропустишь ни дня неделю — получишь 100 💎. Пропустишь — ставка сгорит."));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("50 → 100 💎").clicked() {
                            match app.profile.place_wager(50, 7, t) {
                                Ok(()) => app.notice = Some(ts("🎲 Ставка принята! Неделя без пропусков — и 100 💎 твои.").into()),
                                Err(e) => app.notice = Some(e),
                            }
                        }
                    });
                });
            }
        }
    });
    if !app.profile.remediation.is_empty() {
        card(ui, p, Some(p.red_soft), |ui| {
            let names: Vec<&str> = app.profile.remediation.iter().filter_map(|id| SKILLS.iter().find(|s| s.id == id).map(|s| ts(s.name))).collect();
            ui.label(RichText::new(ts("Работа над ошибками")).strong().color(p.red_dark));
            ui.label(RichText::new(names.join(", ")).color(p.text));
            ui.label(RichText::new(ts("Попадут в начало следующего урока.")).size(12.0).color(p.muted));
        });
    }
}

pub fn home(app: &mut LearningApp, ui: &mut Ui) {
    let p = pal(app);
    top_bar(app, ui, &p);
    let t = now();
    let frontier = app.profile.frontier();
    // Right column: one big button, today's goal, interview.
    // Fixed rectangles: path on the left, the one big button on the right.
    let area = ui.max_rect();
    let top = ui.cursor().min.y;
    let split = (area.max.x - 380.0).max(area.min.x + area.width() * 0.55);
    let side = egui::Rect::from_min_max(pos2(split, top), area.max);
    let rest = egui::Rect::from_min_max(pos2(area.min.x, top), pos2(split, area.max.y));
    ui.painter().rect_filled(side, 0.0, p.bg);
    ui.scope_builder(egui::UiBuilder::new().max_rect(side), |ui| {
        egui::ScrollArea::vertical().id_salt("home_side").auto_shrink([false, false]).show(ui, |ui| {
            egui::Frame::NONE.inner_margin(egui::Margin::same(16)).show(ui, |ui| side_column(app, ui, &p, t, frontier));
        });
    });
    ui.scope_builder(egui::UiBuilder::new().max_rect(rest), |ui| {
        page(ui, &p, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.add_space(16.0);
                let width = ui.available_width();
                let mut clicked: Option<(String, bool)> = None;
                let mut testout: Option<usize> = None;
                for (u, unit) in UNITS.iter().enumerate() {
                    let open = u == UNITS.len() - 1 || app.profile.unit_open(u);
                    let color = if open { unit_color(u) } else { p.grey_dark };
                    ui.vertical_centered(|ui| {
                        egui::Frame::NONE.fill(color).corner_radius(14.0).inner_margin(egui::Margin::symmetric(18, 12)).show(ui, |ui| {
                            ui.set_width((width - 80.0).min(560.0));
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(tr(&format!("Раздел {} · {}", u + 1, ts(unit.title))))
                                            .size(19.0)
                                            .strong()
                                            .color(Color32::WHITE),
                                    );
                                    ui.label(RichText::new(ts(unit.subtitle)).color(Color32::from_white_alpha(220)));
                                });
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    let is_cases = u == UNITS.len() - 1;
                                    let left = SKILLS.iter().filter(|s| s.unit == u).any(|s| app.profile.level(s.id) < 3);
                                    if !is_cases && left {
                                        let b = egui::Button::new(RichText::new(ts("⏩ Я это знаю")).strong().color(Color32::WHITE))
                                            .fill(Color32::from_black_alpha(40))
                                            .corner_radius(10.0);
                                        if ui
                                            .add(b)
                                            .on_hover_text(ts("Проверка на уровне «Сам»: что решишь чисто — засчитаем сразу, раздел откроется"))
                                            .clicked()
                                        {
                                            testout = Some(u);
                                        }
                                    }
                                    if !open {
                                        ui.label(RichText::new(ts("закрыто")).size(14.0).color(Color32::WHITE))
                                            .on_hover_text(ts("Откроется после предыдущих разделов — или пройди «Я это знаю»"));
                                    }
                                });
                            });
                        });
                    });
                    ui.add_space(30.0);
                    // Nodes in a gentle zigzag.
                    let nodes: Vec<(String, String, bool, u8, bool)> = if u == UNITS.len() - 1 {
                        CASES
                            .iter()
                            .map(|c| {
                                (
                                    c.id.to_string(),
                                    ts(c.title).to_string(),
                                    app.profile.case_open(c.id),
                                    app.profile.cases.get(c.id).map(|x| (x.best * 4.0) as u8).unwrap_or(0),
                                    true,
                                )
                            })
                            .collect()
                    } else {
                        SKILLS
                            .iter()
                            .filter(|s| s.unit == u)
                            .map(|s| (s.id.to_string(), ts(s.name).to_string(), app.profile.unlocked(s.id), app.profile.level(s.id), false))
                            .collect()
                    };
                    for (i, (id, name, unlocked, level, is_case)) in nodes.iter().enumerate() {
                        let offset = (i as f32 * 0.95).sin() * 90.0;
                        let (row, _) = ui.allocate_exact_size(vec2(width, 104.0), Sense::hover());
                        let center = pos2(row.center().x + offset, row.min.y + 40.0);
                        let r = 34.0;
                        let hit = egui::Rect::from_center_size(center, vec2(r * 2.0 + 12.0, r * 2.0 + 12.0));
                        let resp = ui.interact(hit, ui.id().with(("node", id)), Sense::click());
                        let is_frontier = frontier == Some(id.as_str());
                        let fill = if *unlocked { unit_color(u) } else { p.grey };
                        let painter = ui.painter();
                        view::level_ring(ui, center, r + 8.0, *level, p.gold, p.grey);
                        let lift = if resp.is_pointer_button_down_on() { 0.0 } else { 5.0 };
                        let bob = if is_frontier { ((ui.ctx().input(|i| i.time) as f32) * 3.0).sin().abs() * 4.0 } else { 0.0 };
                        let c2 = center - vec2(0.0, bob);
                        painter.circle_filled(c2 + vec2(0.0, lift * 0.6), r, view::darker(fill));
                        painter.circle_filled(c2 - vec2(0.0, lift * 0.4), r, fill);
                        let icon = if *is_case {
                            "💼".to_string()
                        } else if !*unlocked {
                            String::new()
                        } else if *level >= 4 {
                            "★".to_string()
                        } else {
                            format!("{}", i + 1)
                        };
                        if !*unlocked && !*is_case {
                            padlock(painter, c2 - vec2(0.0, lift * 0.4), 26.0, Color32::WHITE);
                        }
                        painter.text(c2 - vec2(0.0, lift * 0.4), egui::Align2::CENTER_CENTER, icon, FontId::proportional(24.0), Color32::WHITE);
                        painter.text(
                            pos2(center.x, row.min.y + 92.0),
                            egui::Align2::CENTER_CENTER,
                            name,
                            FontId::proportional(13.0),
                            if *unlocked { p.text } else { p.muted },
                        );
                        if is_frontier {
                            let tag = egui::Rect::from_center_size(c2 - vec2(0.0, r + 24.0), vec2(88.0, 26.0));
                            painter.rect_filled(tag, 8.0, p.bg);
                            painter.rect_stroke(tag, 8.0, Stroke::new(2.0, p.border), egui::StrokeKind::Inside);
                            painter.text(tag.center(), egui::Align2::CENTER_CENTER, ts("НАЧАТЬ"), FontId::proportional(13.0), unit_color(u));
                            ui.ctx().request_repaint();
                        }
                        let tip = if *is_case {
                            format!(
                                "{name}\nРабочий кейс без подсказок. {}",
                                if *unlocked {
                                    ts("Нажми, чтобы начать.")
                                } else {
                                    ts("Откроется, когда нужные навыки будут на уровне 2.")
                                }
                            )
                        } else {
                            let due = app.profile.skills.get(id).filter(|s| s.card.reps > 0).map(|s| {
                                let d = s.card.due.saturating_sub(t) as f32 / 86_400.0;
                                if d <= 0.0 {
                                    ts("Пора повторить").to_string()
                                } else {
                                    tr(&format!("Повтор через {d:.0} дн."))
                                }
                            });
                            format!("{name}\nУровень: {}{}", level_name(*level), due.map(|d| format!("\n{d}")).unwrap_or_default())
                        };
                        let resp = resp.on_hover_text(tip);
                        if *unlocked && resp.clicked() {
                            clicked = Some((id.clone(), *is_case));
                        }
                        if *unlocked && resp.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                    }
                    ui.add_space(18.0);
                }
                if let Some(u) = testout {
                    let r = app.start_testout(u);
                    app.handle(r);
                }
                if let Some((id, case)) = clicked {
                    let r = if case { app.start_case(&id, false) } else { app.start_lesson(Some(&id)) };
                    app.handle(r);
                }
                ui.add_space(30.0);
            });
        })
    });
}

/// A small padlock drawn in code (the emoji is missing from many Linux fonts).
fn padlock(painter: &egui::Painter, c: egui::Pos2, size: f32, color: Color32) {
    let body = egui::Rect::from_center_size(c + vec2(0.0, size * 0.18), vec2(size * 0.8, size * 0.6));
    painter.circle_stroke(c - vec2(0.0, size * 0.12), size * 0.26, Stroke::new(size * 0.1, color));
    painter.rect_filled(body, size * 0.1, color);
}

pub fn lesson(app: &mut LearningApp, ui: &mut Ui) {
    let p = pal(app);
    let Some(a) = app.active.as_ref() else {
        app.screen = Screen::Home;
        return;
    };
    let (done, total, combo) = a.lesson.as_ref().map(|l| (l.done, l.total, l.combo)).unwrap_or((0, 1, 0));
    let sprint = a.lesson.as_ref().and_then(|l| l.sprint.clone());
    let misses = a.lesson.as_ref().map(|l| l.misses).unwrap_or(0);
    let task_secs = a.seconds();
    let slot = a.slot.clone();
    let task = a.task.clone();
    let hints_used = app.show_hints;
    let choosing = a.choosing().is_some();
    let mut quit = false;
    egui::Panel::top("lesson_top").frame(egui::Frame::NONE.fill(p.bg).inner_margin(egui::Margin::symmetric(20, 10))).show(ui, |ui| {
        ui.horizontal(|ui| {
            if ui.add(egui::Button::new(RichText::new("✕").size(20.0).color(p.muted)).frame(false)).on_hover_text(ts("Выйти из урока")).clicked()
            {
                quit = true;
            }
            ui.add_space(8.0);
            let w = ui.available_width() - 150.0;
            let frac = (done as f32 + if app.feedback.as_ref().is_some_and(|f| f.passed) { 1.0 } else { 0.0 }) / total.max(1) as f32;
            progress(ui, frac, if sprint.is_some() { p.orange } else { p.green }, p.grey, vec2(w.max(100.0), 18.0));
            ui.add_space(10.0);
            if let Some(kind) = &sprint {
                let total = app.sprint_elapsed() + misses as f32 * gridcraft_learn_core::profile::SPRINT_PENALTY;
                ui.label(RichText::new(format!("🏁 {}", clock(total))).size(22.0).strong().color(p.orange));
                if let Some(best) = app.profile.sprint_best(kind) {
                    ui.label(RichText::new(tr(&format!("рекорд {}", clock(best)))).size(12.0).color(p.muted));
                }
                ui.ctx().request_repaint();
            } else if combo >= 2 {
                ui.label(RichText::new(tr(&format!("🔥 {combo} подряд"))).strong().color(p.orange));
            }
        });
    });
    if quit {
        app.quit_to_home();
        return;
    }
    egui::Panel::top("task_card")
        .frame(egui::Frame::NONE.fill(p.bg).inner_margin(egui::Margin { left: 24, right: 24, top: 4, bottom: 12 }).stroke(Stroke::new(1.0, p.border)))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                let (tag, fg, bg) = match &slot {
                    Some(Slot::Check(_)) => (ts("СРЕЗ"), Color32::WHITE, Color32::from_rgb(20, 160, 170)),
                    Some(Slot::Review(_)) => (ts("ПОВТОРЕНИЕ"), Color32::WHITE, p.blue),
                    Some(Slot::Mix(_)) => (ts("ВПЕРЕМЕШКУ"), Color32::WHITE, Color32::from_rgb(168, 85, 247)),
                    Some(Slot::Retry(_)) => (ts("РАБОТА НАД ОШИБКОЙ"), Color32::WHITE, p.red),
                    _ if app.profile.level(&task.spec.skill) <= 1 && app.profile.skills.get(&task.spec.skill).is_none_or(|s| s.attempts == 0) => {
                        (ts("НОВОЕ"), Color32::WHITE, p.green)
                    }
                    _ => (ts("ТРЕНИРОВКА"), Color32::WHITE, p.green_dark),
                };
                chip(ui, tag, fg, bg);
                // A skill check and an open method question don't give the tool away.
                if !choosing && !matches!(slot, Some(Slot::Check(_))) {
                    chip(ui, &task.skill_name(), p.text, p.card);
                }
                chip(ui, &tr(&format!("Уровень {}: {}", task.spec.level, level_name(task.spec.level))), p.muted, p.card);
                if !task.keys().is_empty() {
                    chip(ui, &format!("⌨ {}", task.keys().join(", ")), p.text, Color32::from_rgb(255, 236, 179));
                }
                if sprint.is_none() && app.feedback.is_none() {
                    // A quiet stopwatch: under 40 % of the target time is a lightning answer.
                    let goal = task.goal_seconds;
                    let c = if task_secs <= goal * 0.4 {
                        p.gold
                    } else if task_secs <= goal * 0.7 {
                        p.green
                    } else {
                        p.muted
                    };
                    let mark = if task_secs <= goal * 0.4 { "⚡" } else { "⏱" };
                    chip(ui, &tr(&format!("{mark} {:.0} / {:.0} с", task_secs, goal)), c, p.card);
                    ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
                }
            });
            ui.add_space(6.0);
            ui.label(RichText::new(&task.prompt).size(20.0).color(p.text));
            if let Some(ex) = &task.example {
                ui.add_space(6.0);
                egui::Frame::NONE
                    .fill(p.card)
                    .stroke(Stroke::new(2.0, p.border))
                    .corner_radius(10.0)
                    .inner_margin(egui::Margin::symmetric(14, 8))
                    .show(ui, |ui| {
                        ui.label(RichText::new(ex).font(FontId::monospace(22.0)).color(p.blue_dark));
                    });
            }
            for (i, h) in task.hints.iter().take(hints_used as usize).enumerate() {
                ui.label(RichText::new(format!("💡 {}. {h}", i + 1)).size(15.0).color(p.orange));
            }
            if let Some(ai) = &app.ai_result {
                ui.label(RichText::new(format!("🤖 {ai}")).color(p.blue_dark));
            }
            if !app.error_encounters.is_empty() {
                let mut dismiss = false;
                egui::Frame::NONE.fill(p.card).corner_radius(10.0).inner_margin(14).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        mascot(ui, 48.0, Mood::Think, &p);
                        ui.vertical(|ui| {
                            ui.set_width(ui.available_width());
                            for encounter in &app.error_encounters {
                                ui.label(
                                    RichText::new(tr(&format!("Смотри: в {} появилась {}", encounter.cell, encounter.code))).strong().color(p.text),
                                );
                                ui.label(RichText::new(gridcraft_learn_core::errors::explanation(&encounter.code)).color(p.text));
                            }
                            ui.label(RichText::new(ts("Заверши текущую задачу. Следом разберём причину на небольшом упражнении.")).color(p.muted));
                            dismiss = ui.button(ts("Понятно")).clicked();
                        });
                    });
                });
                if dismiss {
                    app.error_encounters.clear();
                }
            }
            ui.add_space(4.0);
            if choice_card(app, ui, &p, true) {
                return;
            }
        });
    if app.screen != Screen::Lesson {
        return;
    }
    if app.feedback.is_some() {
        feedback_banner(app, ui, &p);
    } else if !app.active.as_ref().is_some_and(|a| a.choosing().is_some()) {
        lesson_actions(app, ui, &p);
    }
    if app.screen != Screen::Lesson {
        return;
    }
    if app.active.as_ref().is_some_and(|a| a.choosing().is_some()) {
        // The sheet appears after the choice, so typing 1–4 never lands in a cell.
        return;
    }
    if task.sql.is_some() {
        sql_panel(app, ui, &p);
    } else {
        app.sheet.ui(ui);
    }
}

pub fn clock(secs: f32) -> String {
    format!("{}:{:04.1}", (secs / 60.0) as u32, secs % 60.0)
}

fn sql_values(ui: &mut Ui, p: &Palette, id: &str, header: &[String], rows: &[Vec<String>]) {
    egui::Grid::new(id).striped(true).spacing(vec2(14.0, 4.0)).show(ui, |ui| {
        for h in header {
            ui.label(RichText::new(h).monospace().strong().color(p.text));
        }
        ui.end_row();
        for r in rows {
            for v in r {
                let c = if v == "NULL" { p.muted } else { p.text };
                ui.label(RichText::new(v).monospace().color(c));
            }
            ui.end_row();
        }
    });
}

/// SQL drills: the tables on the left, the editor and the result on the right.
fn sql_panel(app: &mut LearningApp, ui: &mut Ui, p: &Palette) {
    let Some(a) = app.active.as_ref() else { return };
    let Some(task_sql) = a.task.sql.clone() else { return };
    egui::Panel::left("sql_tables")
        .exact_size((ui.available_width() * 0.45).max(320.0))
        .frame(egui::Frame::NONE.fill(p.card).inner_margin(egui::Margin::same(14)))
        .show(ui, |ui| {
            egui::ScrollArea::both().id_salt("sql_tables_scroll").auto_shrink([false, false]).show(ui, |ui| {
                for t in &task_sql.tables {
                    ui.label(RichText::new(format!("🗄 {}", t.name)).size(17.0).strong().color(p.blue_dark));
                    let types: Vec<String> = t.columns.iter().map(|(c, ty)| format!("{c} {ty}")).collect();
                    ui.label(RichText::new(types.join(", ")).size(11.0).color(p.muted));
                    let header: Vec<String> = t.columns.iter().map(|(c, _)| c.clone()).collect();
                    let rows: Vec<Vec<String>> = t.rows.iter().map(|r| r.iter().map(|v| v.show()).collect()).collect();
                    sql_values(ui, p, &format!("tbl_{}", t.name), &header, &rows);
                    ui.add_space(14.0);
                }
            });
        });
    let mut run = false;
    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(p.bg).inner_margin(egui::Margin::same(14))).show(ui, |ui| {
        ui.label(RichText::new(ts("Запрос (SQLite)")).strong().color(p.text));
        if let Some(a) = app.active.as_mut() {
            let edit = egui::TextEdit::multiline(&mut a.query)
                .font(FontId::monospace(18.0))
                .desired_rows(6)
                .desired_width(f32::INFINITY)
                .hint_text("SELECT …")
                .id(egui::Id::new("sql_editor"));
            let resp = ui.add(edit);
            if app.feedback.is_none() && !resp.has_focus() && a.query.is_empty() {
                resp.request_focus();
            }
        }
        ui.horizontal(|ui| {
            if big_button(ui, ts("▶ ВЫПОЛНИТЬ"), Color32::from_rgb(70, 90, 220), vec2(180.0, 42.0)).clicked() {
                run = true;
            }
            ui.label(RichText::new("Ctrl+Enter").size(12.0).color(p.muted));
        });
        ui.add_space(8.0);
        match app.active.as_ref().and_then(|a| a.sql_result.clone()) {
            Some(Ok(rs)) => {
                let note = if rs.truncated { ts(" (показаны первые 2000)") } else { "" };
                ui.label(RichText::new(tr(&format!("Результат: {} строк{note}", rs.rows.len()))).color(p.muted));
                egui::ScrollArea::both().id_salt("sql_result_scroll").auto_shrink([false, false]).show(ui, |ui| {
                    let rows: Vec<Vec<String>> = rs.rows.iter().take(300).map(|r| r.iter().map(|v| v.show()).collect()).collect();
                    sql_values(ui, p, "sql_result", &rs.columns, &rows);
                });
            }
            Some(Err(e)) => {
                ui.label(RichText::new(e).color(p.red_dark));
            }
            None => {
                ui.label(RichText::new(ts("Напиши запрос и выполни — результат появится здесь.")).color(p.muted));
            }
        }
    });
    if run {
        app.run_sql();
    }
}

/// Keep the primary action in the same corner before and after checking.
fn lesson_actions(app: &mut LearningApp, ui: &mut Ui, p: &Palette) {
    let Some(a) = app.active.as_ref() else { return };
    let checkpoint = matches!(a.slot, Some(Slot::Check(_)));
    let is_sql = a.task.sql.is_some();
    let hints_used = app.show_hints;
    let hints_total = a.task.hints.len();
    let mut check = false;
    let mut skip = false;
    let mut hint = false;
    let mut ai = false;
    egui::Panel::bottom("lesson_actions")
        .frame(egui::Frame::NONE.fill(p.bg).stroke(Stroke::new(1.0, p.border)).inner_margin(egui::Margin::symmetric(28, 18)))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    // Reserve the right corner for the primary action, even on smaller windows.
                    ui.set_width((ui.available_width() - 190.0).max(120.0));
                    let note = if checkpoint {
                        ts("Срез: ответ засчитывается с первой попытки. Подсказок нет.")
                    } else if is_sql {
                        ts("Ctrl+Enter — выполнить запрос; верный результат засчитается сам.")
                    } else {
                        ts("Заполни жёлтые ячейки — верный ответ засчитается сам.")
                    };
                    ui.label(RichText::new(note).size(12.0).color(p.muted));
                    ui.horizontal_wrapped(|ui| {
                        if !checkpoint && hints_total > 0 {
                            hint = ui
                                .add_enabled(
                                    (hints_used as usize) < hints_total,
                                    egui::Button::new(tr(&format!("💡 Подсказка ({hints_used}/{hints_total})"))),
                                )
                                .clicked();
                        }
                        skip = ui.button(ts("⏭ Пропустить")).clicked();
                        if app.ai_config.is_some() && !checkpoint {
                            let ok = app.ai_consent && app.ai_pending.is_none() && app.ai_requests < 20;
                            ai = ui
                                .add_enabled(ok, egui::Button::new(ts("🤖 Подсказка AI")))
                                .on_disabled_hover_text(ts("Включи передачу контекста в настройках"))
                                .clicked();
                        }
                    });
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    check = big_button(ui, if checkpoint { ts("СДАТЬ ОТВЕТ") } else { ts("ПРОВЕРИТЬ") }, p.green, vec2(170.0, 52.0))
                        .on_hover_text("Ctrl+Enter")
                        .clicked();
                });
            });
        });
    if check {
        let r = app.check();
        app.handle(r);
    } else if skip {
        let r = app.advance();
        app.handle(r);
    } else if hint {
        app.hint();
    } else if ai {
        app.ask_ai();
    }
}

fn feedback_banner(app: &mut LearningApp, ui: &mut Ui, p: &Palette) {
    let Some(fb) = app.feedback.as_ref() else { return };
    let passed = fb.passed;
    let (bg, fg) = if passed { (p.green_soft, p.green_dark) } else { (p.red_soft, p.red_dark) };
    let mut next = false;
    let mut fix = false;
    egui::Panel::bottom("feedback").frame(egui::Frame::NONE.fill(bg).inner_margin(egui::Margin::symmetric(28, 18))).show(ui, |ui| {
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(vec2(64.0, 64.0), Sense::hover());
            ui.painter().circle_filled(rect.center(), 30.0, p.bg);
            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, if passed { "✔" } else { "✖" }, FontId::proportional(30.0), fg);
            ui.add_space(8.0);
            ui.vertical(|ui| {
                let actions_width = if passed { 190.0 } else { 350.0 };
                ui.set_width((ui.available_width() - actions_width).max(140.0));
                let title = if fb.corrected {
                    ts("Исправлено!")
                } else if passed {
                    [ts("Отлично!"), ts("Верно!"), ts("Так держать!"), ts("Супер!"), ts("Чётко!")][(fb.eval.outcomes.len() + fb.xp as usize) % 5]
                } else {
                    ts("Не совсем")
                };
                ui.label(RichText::new(title).size(24.0).strong().color(fg));
                if passed {
                    let ch = &fb.change;
                    let mut line = if fb.xp > 0 { format!("+{} XP", fb.xp) } else { String::new() };
                    if ch.speed_bonus > 0 {
                        line.push_str(&tr(&format!("  (⚡ скорость +{}", ch.speed_bonus)));
                        line.push_str(if ch.combo_bonus > 0 { "" } else { ")" });
                    }
                    if ch.combo_bonus > 0 {
                        line.push_str(&tr(&format!("{}🔥 комбо +{})", if ch.speed_bonus > 0 { ", " } else { "  (" }, ch.combo_bonus)));
                    }
                    if ch.fast_track {
                        line.push_str(ts("   ⏩ Ты это знаешь — сразу уровень «Сам»"));
                    }
                    if let Some(l) = fb.level_up {
                        line.push_str(&tr(&format!("   ⬆ Новый уровень: «{}»", level_name(l))));
                    }
                    if fb.goal_reached {
                        line.push_str(ts("   🎯 Цель дня выполнена!"));
                    }
                    if !line.is_empty() {
                        ui.label(RichText::new(line).size(16.0).strong().color(fg));
                    }
                } else {
                    for o in fb.eval.outcomes.iter().filter(|o| !o.passed).take(3) {
                        ui.label(RichText::new(&o.feedback).size(15.0).color(p.text));
                    }
                    if let Some(sol) = fb.eval.outcomes.iter().find(|o| !o.passed).and_then(|o| o.solution.clone()) {
                        ui.label(RichText::new(tr(&format!("Правильно: {sol}"))).font(FontId::monospace(16.0)).color(fg));
                    }
                    if fb.level_down {
                        ui.label(RichText::new(ts("Сделаем шаг поменьше — уровень понижен, это нормально.")).size(13.0).color(p.muted));
                    }
                }
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if big_button(ui, ts("ДАЛЕЕ"), if passed { p.green } else { p.red }, vec2(170.0, 52.0)).clicked() {
                    next = true;
                }
                if !passed && ghost_button(ui, ts("ИСПРАВИТЬ"), p, vec2(150.0, 52.0)).clicked() {
                    fix = true;
                }
                ui.label(RichText::new("Enter").size(12.0).color(p.muted));
            });
        });
    });
    if next {
        let r = app.advance();
        app.handle(r);
    } else if fix {
        app.fix();
    }
}

pub fn case(app: &mut LearningApp, ui: &mut Ui) {
    let p = pal(app);
    let Some(a) = app.active.as_ref() else {
        app.screen = Screen::Home;
        return;
    };
    let task = a.task.clone();
    let interview = a.interview.clone();
    let secs = a.seconds();
    let hints_used = a.hints;
    let mut submit = false;
    let mut quit = false;
    egui::Panel::top("case_top")
        .frame(egui::Frame::NONE.fill(p.bg).inner_margin(egui::Margin::symmetric(20, 10)).stroke(Stroke::new(2.0, p.border)))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .add(egui::Button::new(RichText::new("✕").size(20.0).color(p.muted)).frame(false))
                    .on_hover_text(ts("Выйти без оценки"))
                    .clicked()
                {
                    quit = true;
                }
                let kind = if interview.is_some() { ts("🎯 Собеседование") } else { ts("💼 Рабочий кейс") };
                ui.label(RichText::new(format!("{kind}: {}", tr(&task.title))).size(20.0).strong().color(p.text));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if big_button(ui, ts("СДАТЬ РАБОТУ"), p.green, vec2(180.0, 44.0)).clicked() {
                        submit = true;
                    }
                    ui.add_space(12.0);
                    if let Some(iv) = &interview {
                        let left = (iv.minutes as f32 * 60.0 - secs).max(0.0);
                        let c = if left < 120.0 { p.red } else { p.text };
                        ui.label(RichText::new(format!("⏱ {:02}:{:02}", (left / 60.0) as u32, (left % 60.0) as u32)).size(22.0).strong().color(c));
                    } else {
                        ui.label(RichText::new(format!("⏱ {:02}:{:02}", (secs / 60.0) as u32, (secs % 60.0) as u32)).size(18.0).color(p.muted));
                    }
                });
            });
        });
    if quit {
        app.quit_to_home();
        return;
    }
    egui::Panel::left("case_brief").exact_size(340.0).frame(egui::Frame::NONE.fill(p.card).inner_margin(egui::Margin::same(16))).show(ui, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            if choice_card(app, ui, &p, false) {
                ui.add_space(10.0);
            }
            if let Some(b) = &task.brief {
                egui::Frame::NONE.fill(p.bg).stroke(Stroke::new(1.0, p.border)).corner_radius(10.0).inner_margin(egui::Margin::same(12)).show(
                    ui,
                    |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new(format!("✉ {}", b.from)).strong().color(p.text));
                        ui.label(RichText::new(tr(&format!("Тема: {}", b.subject))).color(p.muted));
                        ui.separator();
                        ui.label(RichText::new(&b.body).size(15.0).color(p.text));
                    },
                );
            }
            ui.add_space(10.0);
            ui.label(RichText::new(ts("Что сдать")).size(17.0).strong().color(p.text));
            ui.label(RichText::new(&task.prompt).size(15.0).color(p.text));
            ui.add_space(8.0);
            ui.label(RichText::new(ts("Ответы — в жёлтые ячейки. Способ любой: формулы, сводная, фильтр.")).size(12.0).color(p.muted));
            if interview.is_none() {
                ui.add_space(8.0);
                if hints_used < task.hints.len() as u32 && ui.button(ts("💡 Подсказка")).clicked() {
                    app.hint();
                }
                for h in task.hints.iter().take(hints_used as usize) {
                    ui.label(RichText::new(format!("💡 {h}")).color(p.orange));
                }
            } else {
                ui.add_space(8.0);
                ui.label(
                    RichText::new(ts("Подсказок нет — как на настоящем тестовом. Время вышло — работа сдаётся сама.")).size(12.0).color(p.muted),
                );
            }
        });
    });
    if submit {
        let r = app.submit_case(false);
        app.handle(r);
        return;
    }
    app.sheet.ui(ui);
}

fn tile(ui: &mut Ui, p: &Palette, title: &str, value: &str, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(136.0, 78.0), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 14.0, color);
    painter.text(pos2(rect.center().x, rect.min.y + 13.0), egui::Align2::CENTER_CENTER, title, FontId::proportional(12.0), Color32::WHITE);
    let inner = egui::Rect::from_min_max(pos2(rect.min.x + 3.0, rect.min.y + 26.0), rect.max - vec2(3.0, 3.0));
    painter.rect_filled(inner, 11.0, p.bg);
    painter.text(inner.center(), egui::Align2::CENTER_CENTER, value, FontId::proportional(21.0), color);
}

pub fn report(app: &mut LearningApp, ui: &mut Ui) {
    let p = pal(app);
    let mut action = 0;
    page(ui, &p, |ui| {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(30.0);
                let Some(r) = &app.report else { return };
                let score = r.eval.score;
                mascot(
                    ui,
                    120.0,
                    if score >= 0.8 {
                        Mood::Cheer
                    } else if score >= 0.5 {
                        Mood::Think
                    } else {
                        Mood::Sad
                    },
                    &p,
                );
                let verdict = if score >= 0.999 {
                    ts("Работа принята без замечаний!")
                } else if score >= 0.8 {
                    ts("Хорошо! Пара мелочей.")
                } else if score >= 0.5 {
                    ts("Половина есть. Разберём, что не так.")
                } else {
                    ts("Пока сложно — потренируем нужные навыки.")
                };
                ui.label(RichText::new(format!("{:.0}%", score * 100.0)).size(54.0).strong().color(if score >= 0.8 {
                    p.green
                } else if score >= 0.5 {
                    p.orange
                } else {
                    p.red
                }));
                ui.label(RichText::new(verdict).size(22.0).strong().color(p.text));
                let mins = (r.seconds / 60.0).ceil();
                let kind = if r.transfer {
                    ts("Перенос в Excel")
                } else if r.interview {
                    ts("Собеседование")
                } else {
                    ts("Кейс")
                };
                ui.label(
                    RichText::new(tr(&format!(
                        "{kind} «{}» · {mins} мин · +{} XP{}",
                        r.task.title,
                        r.xp,
                        if r.timed_out { ts(" · время вышло") } else { "" }
                    )))
                    .color(p.muted),
                );
                ui.add_space(16.0);
                egui::Frame::NONE.stroke(Stroke::new(2.0, p.border)).corner_radius(14.0).inner_margin(egui::Margin::same(16)).show(ui, |ui| {
                    ui.set_width(720.0f32.min(ui.available_width() - 40.0));
                    ui.with_layout(Layout::top_down(Align::Min), |ui| {
                        for o in &r.eval.outcomes {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new(if o.passed { "✔" } else { "✖" }).size(18.0).color(if o.passed { p.green } else { p.red }));
                                ui.label(RichText::new(&o.label).strong().color(p.text));
                                ui.label(RichText::new(&o.feedback).color(p.muted));
                            });
                            if !o.passed
                                && let Some(s) = &o.solution
                            {
                                ui.label(RichText::new(tr(&format!("    Ожидалось: {s}"))).font(FontId::monospace(14.0)).color(p.blue_dark));
                            }
                        }
                    });
                });
                ui.add_space(20.0);
                if !app.profile.remediation.is_empty() && big_button(ui, ts("ПОТРЕНИРОВАТЬ ОШИБКИ"), p.green, vec2(320.0, 54.0)).clicked()
                {
                    action = 1;
                }
                ui.add_space(8.0);
                if r.interview && big_button(ui, ts("ЕЩЁ ОДНО СОБЕСЕДОВАНИЕ"), p.blue, vec2(320.0, 50.0)).clicked() {
                    action = 2;
                }
                if r.transfer && !r.eval.passed() {
                    ui.label(RichText::new(ts("Исправь файл в Excel, сохрани и проверь ещё раз — задание ждёт на главной.")).color(p.muted));
                }
                ui.add_space(8.0);
                if ghost_button(ui, ts("НА ГЛАВНУЮ"), &p, vec2(320.0, 46.0)).clicked() {
                    action = 3;
                }
                ui.add_space(30.0);
            });
        });
    });
    match action {
        1 => {
            let r = app.start_lesson(None);
            app.handle(r);
        }
        2 => {
            let r = app.start_interview();
            app.handle(r);
        }
        3 => app.screen = Screen::Home,
        _ => {}
    }
}

pub fn done(app: &mut LearningApp, ui: &mut Ui) {
    let p = pal(app);
    let mut more = false;
    let mut again = false;
    let mut home = false;
    // Numbers count up during the first second on this screen.
    let k: f32 = app.done_since.map(|t| (app.time() - t).clamp(0.0, 1.0) as f32).unwrap_or(1.0);
    if k < 1.0 {
        ui.ctx().request_repaint();
    }
    let up = |v: f32| v * (1.0 - (1.0 - k).powi(3));
    let sprint_kind = app.summary.as_ref().and_then(|s| s.sprint.as_ref().map(|x| x.kind.clone()));
    page(ui, &p, |ui| {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(30.0);
                let bounce = ((app.time() * 4.0).sin().abs() * 8.0) as f32 * (1.0 - k * 0.7);
                ui.add_space(bounce);
                mascot(ui, 140.0, Mood::Cheer, &p);
                ui.add_space(10.0 - bounce.min(10.0));
                let Some(s) = &app.summary else { return };
                if let Some(c) = &s.checkpoint {
                    ui.label(RichText::new(tr(&format!("Срез «{}» пройден", ts(c.phase.label())))).size(34.0).strong().color(p.text));
                    ui.label(RichText::new(format!("{} / {}", c.solved, c.total)).size(56.0).strong().color(Color32::from_rgb(20, 160, 170)));
                    ui.label(RichText::new(tr(&format!("за {}", clock(c.seconds)))).size(18.0).color(p.muted));
                    let earlier = app.profile.checkpoints.iter().rev().nth(1);
                    if let Some(e) = earlier {
                        let d = c.solved as i64 - e.solved as i64;
                        let text = match d {
                            d if d > 0 => tr(&format!("На {d} больше, чем на срезе «{}». Навык растёт!", ts(e.phase.label()))),
                            0 => tr(&format!("Столько же, сколько на срезе «{}».", ts(e.phase.label()))),
                            d => tr(&format!("На {} меньше, чем на срезе «{}» — повтори слабые навыки.", -d, ts(e.phase.label()))),
                        };
                        ui.label(RichText::new(text).size(18.0).strong().color(if d >= 0 { p.green_dark } else { p.orange }));
                    }
                    let next = match c.phase {
                        gridcraft_learn_core::profile::Phase::Before => {
                            ts("Теперь учись. Срез «После обучения» откроется, когда 5 из 8 навыков дойдут до уровня «Сам».")
                        }
                        gridcraft_learn_core::profile::Phase::After => ts("Через неделю — отложенный срез: проверим, что навык не выветрился."),
                        gridcraft_learn_core::profile::Phase::Later => ts("Цикл замеров завершён. Все результаты — в статистике."),
                    };
                    ui.label(RichText::new(next).size(14.0).color(p.muted));
                } else if let Some(sp) = &s.sprint {
                    ui.label(RichText::new(ts("Спринт завершён!")).size(34.0).strong().color(p.orange));
                    ui.label(RichText::new(clock(up(sp.seconds))).size(56.0).strong().color(p.text));
                    let mut line = sp.medal.label().to_string();
                    if sp.misses > 0 {
                        line.push_str(&tr(&format!(
                            " · ошибок {} (+{:.0} с штрафа)",
                            sp.misses,
                            sp.misses as f32 * gridcraft_learn_core::profile::SPRINT_PENALTY
                        )));
                    }
                    ui.label(RichText::new(line).size(20.0).strong().color(p.gold));
                    match sp.previous {
                        Some(prev) if sp.record => {
                            ui.label(
                                RichText::new(tr(&format!("🏁 НОВЫЙ РЕКОРД! Было {}, стало быстрее на {:.1} с", clock(prev), prev - sp.seconds)))
                                    .size(18.0)
                                    .strong()
                                    .color(p.green_dark),
                            );
                        }
                        Some(prev) => {
                            ui.label(RichText::new(tr(&format!("Рекорд {} — до него {:.1} с", clock(prev), sp.seconds - prev))).color(p.muted));
                        }
                        None => {
                            ui.label(RichText::new(ts("Первый заезд — это твой рекорд. Теперь побей его!")).color(p.muted));
                        }
                    }
                    let gold = sp.par * 0.4;
                    ui.label(RichText::new(tr(&format!("Золото — быстрее {}", clock(gold)))).size(13.0).color(p.muted));
                } else {
                    let title = if s.test_out {
                        ts("Проверка пройдена!")
                    } else if s.perfect {
                        ts("Идеальный урок!")
                    } else {
                        ts("Урок пройден!")
                    };
                    ui.label(RichText::new(title).size(34.0).strong().color(p.gold));
                    if s.perfect {
                        ui.label(RichText::new(ts("💎 Ни одной ошибки — бонус +10 💎")).size(16.0).color(p.blue));
                    }
                }
                if s.streak_extended {
                    ui.label(RichText::new(tr(&format!("🔥 {} дн. подряд!", s.streak))).size(20.0).strong().color(p.orange));
                }
                ui.add_space(16.0);
                if s.checkpoint.is_none() {
                    ui.horizontal(|ui| {
                        let w = 4.0 * 136.0 + 3.0 * 12.0;
                        ui.add_space(((ui.available_width() - w) / 2.0).max(0.0));
                        tile(ui, &p, ts("ОПЫТ"), &format!("⚡ {:.0}", up(s.xp as f32)), p.gold);
                        ui.add_space(12.0);
                        tile(ui, &p, ts("ТОЧНОСТЬ"), &format!("🎯 {:.0}%", up(s.accuracy * 100.0)), p.green);
                        ui.add_space(12.0);
                        tile(ui, &p, ts("КРИСТАЛЛЫ"), &format!("💎 {:.0}", up(s.gems as f32)), p.blue);
                        ui.add_space(12.0);
                        tile(ui, &p, ts("КОМБО"), &format!("🔥 {:.0}", up(s.best_combo as f32)), p.orange);
                    });
                }
                for l in &s.level_ups {
                    ui.add_space(6.0);
                    ui.label(RichText::new(format!("⬆ {l}")).size(17.0).strong().color(p.green_dark));
                }
                ui.add_space(14.0);
                let today = app.profile.today_xp(now());
                ui.label(RichText::new(tr(&format!("Цель дня: {today} / {} XP", app.profile.goal))).color(p.muted));
                ui.add_space(20.0);
                if s.sprint.is_some() {
                    if big_button(ui, ts("ЕЩЁ ЗАЕЗД"), p.orange, vec2(300.0, 56.0)).clicked() {
                        again = true;
                    }
                } else if big_button(ui, ts("ЕЩЁ УРОК"), p.green, vec2(300.0, 56.0)).clicked() {
                    more = true;
                }
                ui.add_space(8.0);
                if ghost_button(ui, ts("НА ГЛАВНУЮ"), &p, vec2(300.0, 48.0)).clicked() {
                    home = true;
                }
                ui.label(RichText::new(ts("Enter — продолжить")).size(12.0).color(p.muted));
            });
        });
    });
    if again {
        let kind = sprint_kind.unwrap_or_else(|| "mix".into());
        let r = app.start_sprint(if kind == "mix" { None } else { Some(&kind) });
        app.handle(r);
    } else if more {
        let r = app.start_lesson(None);
        app.handle(r);
    } else if home {
        app.screen = Screen::Home;
    }
}

pub fn blitz(app: &mut LearningApp, ui: &mut Ui) {
    let p = pal(app);
    let Some(b) = app.blitz.as_ref() else {
        app.screen = Screen::Home;
        return;
    };
    let Some(q) = b.current().cloned() else { return };
    let answered = b.answered();
    let used = b.used();
    let index = b.index;
    let total = b.questions.len();
    let points = b.points();
    let combo = b.combo;
    let track = b.track;
    let last = b.answers.get(index).cloned();
    let mut quit = false;
    let mut choice: Option<usize> = None;
    let mut next = false;
    page(ui, &p, |ui| {
        egui::Frame::NONE.inner_margin(egui::Margin::symmetric(40, 20)).show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new(RichText::new("✕").size(20.0).color(p.muted)).frame(false)).clicked() {
                    quit = true;
                }
                ui.label(RichText::new(tr(&format!("⏱ Экспресс-тест · {}", track.title()))).size(18.0).strong().color(p.text));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(tr(&format!("{points} очков"))).size(18.0).strong().color(p.gold));
                    if combo >= 2 {
                        ui.label(RichText::new(format!("🔥 ×{combo}")).strong().color(p.orange));
                    }
                    ui.label(RichText::new(format!("{} / {total}", index + 1)).color(p.muted));
                });
            });
            progress(ui, (index as f32 + if answered { 1.0 } else { 0.0 }) / total.max(1) as f32, p.blue, p.grey, vec2(ui.available_width(), 12.0));
            ui.add_space(8.0);
            // The question's own timer.
            let left = (q.seconds as f32 - used).max(0.0);
            let frac = if answered { last.as_ref().map(|a| 1.0 - a.1 / q.seconds as f32).unwrap_or(0.0) } else { left / q.seconds as f32 };
            let c = if frac > 0.5 {
                p.green
            } else if frac > 0.2 {
                p.orange
            } else {
                p.red
            };
            ui.horizontal(|ui| {
                progress(ui, frac, c, p.grey, vec2(ui.available_width() - 70.0, 10.0));
                ui.label(RichText::new(tr(&format!("{left:.0} с"))).strong().color(c));
            });
            if !answered {
                ui.ctx().request_repaint();
            }
            ui.add_space(14.0);
            chip(ui, &q.topic, p.text, p.card);
            ui.add_space(6.0);
            let (text, code) = match q.prompt.split_once('\n') {
                Some((a, b)) => (a.to_string(), Some(b.to_string())),
                None => (q.prompt.clone(), None),
            };
            ui.label(RichText::new(text).size(24.0).strong().color(p.text));
            if let Some(code) = code {
                egui::Frame::NONE.fill(p.card).corner_radius(10.0).inner_margin(egui::Margin::symmetric(14, 8)).show(ui, |ui| {
                    ui.label(RichText::new(code).font(FontId::monospace(17.0)).color(p.blue_dark));
                });
            }
            if !q.table.is_empty() {
                ui.add_space(8.0);
                egui::Frame::NONE.stroke(Stroke::new(1.0, p.border)).corner_radius(8.0).inner_margin(egui::Margin::same(10)).show(ui, |ui| {
                    egui::Grid::new(("blitz_table", index)).striped(true).spacing(vec2(18.0, 4.0)).show(ui, |ui| {
                        for (r, row) in q.table.iter().enumerate() {
                            for v in row {
                                let t = RichText::new(v).monospace().color(p.text);
                                ui.label(if r == 0 { t.strong() } else { t });
                            }
                            ui.end_row();
                        }
                    });
                });
            }
            ui.add_space(16.0);
            let w = ui.available_width().min(760.0);
            for (i, o) in q.options.iter().enumerate() {
                let picked = last.as_ref().is_some_and(|a| a.0 == Some(i));
                let fill = if answered && i == q.correct {
                    p.green
                } else if picked {
                    p.red
                } else {
                    p.grey_dark
                };
                let label = format!("{}   {o}", i + 1);
                let resp = if answered { view::big_button(ui, &label, fill, vec2(w, 50.0)) } else { option_button(ui, &label, &p, vec2(w, 50.0)) };
                if resp.clicked() {
                    if answered {
                        next = true;
                    } else {
                        choice = Some(i);
                    }
                }
                ui.add_space(8.0);
            }
            if let Some(a) = &last {
                let right = a.0 == Some(q.correct);
                let head = if right {
                    tr(&format!("✔ Верно! +{} очков за {:.1} с", a.2, a.1))
                } else if a.0.is_none() {
                    ts("⏰ Время вышло").to_string()
                } else {
                    ts("✖ Неверно").to_string()
                };
                ui.label(RichText::new(head).size(18.0).strong().color(if right { p.green_dark } else { p.red_dark }));
                ui.label(RichText::new(&q.explain).color(p.text));
                ui.add_space(8.0);
                if big_button(ui, ts("ДАЛЕЕ"), p.blue, vec2(220.0, 48.0)).clicked() {
                    next = true;
                }
                ui.label(RichText::new(ts("Enter или любая цифра")).size(12.0).color(p.muted));
            } else {
                ui.label(RichText::new(ts("Отвечай клавишами 1–4 — так быстрее.")).size(12.0).color(p.muted));
            }
        });
    });
    if quit {
        app.blitz = None;
        app.screen = Screen::Home;
    } else if let Some(c) = choice {
        app.blitz_answer(Some(c));
    } else if next {
        app.blitz_next();
    }
}

/// The "which method and why" card. Returns true while it is on screen.
fn choice_card(app: &mut LearningApp, ui: &mut Ui, p: &Palette, wide: bool) -> bool {
    let Some(a) = app.active.as_ref() else { return false };
    let Some(c) = a.choosing().cloned() else { return false };
    let picked = a.picked;
    let (step, steps) = (a.choice + 1, a.task.choices.len());
    let mut pick = None;
    let mut next = false;
    egui::Frame::NONE.fill(p.card).stroke(Stroke::new(2.0, p.blue)).corner_radius(12.0).inner_margin(egui::Margin::same(12)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        let head = if steps > 1 { format!("{} ({step}/{steps})", c.question) } else { c.question.clone() };
        ui.label(RichText::new(head).size(18.0).strong().color(p.text));
        ui.add_space(6.0);
        let w = if wide { ui.available_width().min(900.0) } else { ui.available_width() };
        for (i, o) in c.options.iter().enumerate() {
            let fill = match picked {
                Some(_) if o.ok => p.green,
                Some(k) if k == i => p.red,
                _ => p.bg,
            };
            let fg = if picked.is_some() && (o.ok || picked == Some(i)) { Color32::WHITE } else { p.text };
            let b = egui::Button::new(RichText::new(format!("{}. {}", i + 1, o.text)).size(15.0).color(fg)).fill(fill).wrap().min_size(vec2(w, 34.0));
            let r = ui.add_enabled(picked.is_none() || o.ok || picked == Some(i), b);
            if r.clicked() && picked.is_none() {
                pick = Some(i);
            }
            // Keys only where the sheet is hidden, so typing into a cell never picks.
            if wide && picked.is_none() && ui.input(|inp| inp.key_pressed(egui::Key::from_name(&(i + 1).to_string()).unwrap_or(egui::Key::Escape))) {
                pick = Some(i);
            }
        }
        if let Some(k) = picked
            && let Some(o) = c.options.get(k)
        {
            ui.add_space(6.0);
            let color = if o.ok { p.green_dark } else { p.red };
            ui.label(RichText::new(format!("{} {}", if o.ok { "✔" } else { "✗" }, o.why)).size(15.0).color(color));
            if !o.ok
                && let Some(right) = c.options.iter().find(|o| o.ok)
            {
                ui.label(RichText::new(&right.why).size(14.0).color(p.text));
            }
            ui.add_space(4.0);
            let label = if wide { ts("Дальше  (Enter)") } else { ts("Дальше") };
            if ui.button(RichText::new(label).size(15.0)).clicked() || (wide && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                next = true;
            }
        } else {
            let note = if wide {
                ts("Выбери вариант мышью или клавишами 1–4.")
            } else {
                ts("Сначала выбери способ — потом решай.")
            };
            ui.label(RichText::new(note).size(12.0).color(p.muted));
        }
    });
    if let Some(i) = pick {
        app.pick_choice(i);
    }
    if next {
        app.next_choice();
    }
    true
}

fn option_button(ui: &mut Ui, text: &str, p: &Palette, size: egui::Vec2) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let painter = ui.painter();
    let hover = resp.hovered();
    painter.rect_filled(rect.translate(vec2(0.0, 3.0)), 12.0, p.border);
    painter.rect_filled(rect, 12.0, if hover { p.card } else { p.bg });
    painter.rect_stroke(rect, 12.0, Stroke::new(2.0, if hover { p.blue } else { p.border }), egui::StrokeKind::Inside);
    painter.text(rect.left_center() + vec2(18.0, 0.0), egui::Align2::LEFT_CENTER, text, FontId::proportional(17.0), p.text);
    if hover {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

pub fn blitz_report(app: &mut LearningApp, ui: &mut Ui) {
    let p = pal(app);
    let mut action = 0;
    page(ui, &p, |ui| {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(24.0);
                let Some((b, change)) = &app.blitz_result else { return };
                let score = b.correct() as f32 / b.questions.len().max(1) as f32;
                mascot(
                    ui,
                    110.0,
                    if score >= 0.8 {
                        Mood::Cheer
                    } else if score >= 0.5 {
                        Mood::Think
                    } else {
                        Mood::Sad
                    },
                    &p,
                );
                ui.label(RichText::new(format!("{} / {}", b.correct(), b.questions.len())).size(52.0).strong().color(if score >= 0.8 {
                    p.green
                } else if score >= 0.5 {
                    p.orange
                } else {
                    p.red
                }));
                let secs = b.began.elapsed().as_secs_f32();
                ui.label(
                    RichText::new(tr(&format!(
                        "{} очков · {} · +{} XP",
                        b.points(),
                        clock(secs.min(b.answers.iter().map(|a| a.1).sum::<f32>())),
                        change.xp
                    )))
                    .size(18.0)
                    .color(p.text),
                );
                let verdict = if score >= 0.9 {
                    ts("Уровень отбора пройден с запасом.")
                } else if score >= 0.7 {
                    ts("Хорошо! Добей ошибки — и тест сдан.")
                } else {
                    ts("Ошибки ушли в «работу над ошибками» — их потренируем в уроке.")
                };
                ui.label(RichText::new(verdict).color(p.muted));
                ui.add_space(14.0);
                egui::Frame::NONE.stroke(Stroke::new(2.0, p.border)).corner_radius(14.0).inner_margin(egui::Margin::same(14)).show(ui, |ui| {
                    ui.set_width(760.0f32.min(ui.available_width() - 40.0));
                    ui.with_layout(Layout::top_down(Align::Min), |ui| {
                        for (q, a) in b.questions.iter().zip(&b.answers) {
                            let right = a.0 == Some(q.correct);
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new(if right { "✔" } else { "✖" }).color(if right { p.green } else { p.red }));
                                ui.label(RichText::new(q.prompt.lines().next().unwrap_or("")).color(p.text));
                                ui.label(RichText::new(tr(&format!("{:.1} с", a.1))).size(12.0).color(p.muted));
                            });
                            if !right {
                                let answer = q.options.get(q.correct).cloned().unwrap_or_default();
                                let text =
                                    if q.explain.contains(&answer) { q.explain.clone() } else { tr(&format!("Ответ: {answer}. {}", q.explain)) };
                                ui.label(RichText::new(format!("    {text}")).size(13.0).color(p.blue_dark));
                            }
                        }
                    });
                });
                ui.add_space(16.0);
                if big_button(ui, ts("ЕЩЁ ТЕСТ"), p.blue, vec2(300.0, 52.0)).clicked() {
                    action = 1;
                }
                ui.add_space(8.0);
                if !app.profile.remediation.is_empty() && big_button(ui, ts("ПОТРЕНИРОВАТЬ ОШИБКИ"), p.green, vec2(300.0, 52.0)).clicked()
                {
                    action = 2;
                }
                ui.add_space(8.0);
                if ghost_button(ui, ts("НА ГЛАВНУЮ"), &p, vec2(300.0, 46.0)).clicked() {
                    action = 3;
                }
                ui.add_space(24.0);
            });
        });
    });
    match action {
        1 => {
            let track = app.blitz_result.as_ref().map(|(b, _)| b.track).unwrap_or(Track::Mixed);
            app.start_blitz(track);
        }
        2 => {
            let r = app.start_lesson(None);
            app.handle(r);
        }
        3 => app.screen = Screen::Home,
        _ => {}
    }
}

/// Longest run of consecutive practice days.
fn best_streak(days: &std::collections::BTreeMap<u64, u32>) -> u32 {
    let (mut best, mut run, mut prev) = (0u32, 0u32, None::<u64>);
    for (d, xp) in days {
        if *xp == 0 {
            continue;
        }
        run = if prev.is_some_and(|p| p + 1 == *d) { run + 1 } else { 1 };
        best = best.max(run);
        prev = Some(*d);
    }
    best
}

pub fn stats(app: &mut LearningApp, ui: &mut Ui) {
    let p = pal(app);
    let mut back = false;
    let mut sprint: Option<Option<String>> = None;
    let t = now();
    page(ui, &p, |ui| {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            egui::Frame::NONE.inner_margin(egui::Margin::same(24)).show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button(ts("← Назад")).clicked() {
                        back = true;
                    }
                    ui.label(RichText::new(ts("Прогресс")).size(28.0).strong().color(p.text));
                });
                ui.add_space(10.0);
                let prof = &app.profile;
                let tries: Vec<_> = prof.log.iter().filter(|a| a.mode == Mode::Drill).collect();
                let ok = tries.iter().filter(|a| a.passed).count();
                ui.horizontal_wrapped(|ui| {
                    tile(ui, &p, ts("СЕРИЯ"), &format!("🔥 {}", prof.current_streak(t)), p.orange);
                    ui.add_space(10.0);
                    tile(ui, &p, ts("ЛУЧШАЯ СЕРИЯ"), &format!("🏆 {}", best_streak(&prof.days)), p.gold);
                    ui.add_space(10.0);
                    tile(ui, &p, ts("ОПЫТ"), &format!("⚡ {}", prof.xp), p.gold);
                    ui.add_space(10.0);
                    tile(ui, &p, ts("КРИСТАЛЛЫ"), &format!("💎 {}", prof.gems), p.blue);
                    ui.add_space(10.0);
                    tile(ui, &p, ts("ТОЧНОСТЬ"), &format!("🎯 {:.0}%", ok as f32 * 100.0 / tries.len().max(1) as f32), p.green);
                    ui.add_space(10.0);
                    tile(ui, &p, ts("МОЛНИИ"), &format!("⚡ {}", prof.lightning), Color32::from_rgb(168, 85, 247));
                    ui.add_space(10.0);
                    tile(ui, &p, ts("ПУТЬ"), &format!("👑 {:.0}%", prof.mastery() * 100.0), p.green_dark);
                });
                ui.add_space(16.0);
                ui.label(RichText::new(ts("Опыт за 14 дней")).size(18.0).strong().color(p.text));
                let today = prof.day(t);
                let names = [ts("пн"), ts("вт"), ts("ср"), ts("чт"), ts("пт"), ts("сб"), ts("вс")];
                let values: Vec<(String, f32)> = (0..14u64)
                    .rev()
                    .map(|i| {
                        let d = today.saturating_sub(i);
                        // Day 0 (1 Jan 1970) was a Thursday.
                        let name = names.get(((d + 3) % 7) as usize).copied().unwrap_or("");
                        (name.to_string(), prof.days.get(&d).copied().unwrap_or(0) as f32)
                    })
                    .collect();
                view::bars(ui, &values, Some(prof.goal as f32), p.gold, &p, 120.0);
                ui.add_space(12.0);
                ui.label(RichText::new(ts("Календарь (5 недель)")).size(18.0).strong().color(p.text));
                let (rect, _) = ui.allocate_exact_size(vec2(7.0 * 34.0, 5.0 * 34.0 + 18.0), Sense::hover());
                let painter = ui.painter();
                for (i, n) in names.iter().enumerate() {
                    painter.text(
                        pos2(rect.min.x + i as f32 * 34.0 + 15.0, rect.min.y + 7.0),
                        egui::Align2::CENTER_CENTER,
                        *n,
                        FontId::proportional(11.0),
                        p.muted,
                    );
                }
                let monday = today - (today + 3) % 7;
                for w in 0..5u64 {
                    for d in 0..7u64 {
                        let day = monday + d + 7 * w - 28;
                        let c = egui::Rect::from_min_size(pos2(rect.min.x + d as f32 * 34.0, rect.min.y + 18.0 + w as f32 * 34.0), vec2(30.0, 30.0));
                        let xp = prof.days.get(&day).copied().unwrap_or(0);
                        let fill = if day > today {
                            p.bg
                        } else if prof.frozen_days.contains(&day) {
                            p.blue
                        } else if xp >= prof.goal {
                            p.orange
                        } else if xp > 0 {
                            p.gold.gamma_multiply(0.5)
                        } else {
                            p.grey
                        };
                        painter.rect_filled(c, 8.0, fill);
                        if day == today {
                            painter.rect_stroke(c, 8.0, Stroke::new(2.0, p.text), egui::StrokeKind::Inside);
                        }
                        if xp >= prof.goal {
                            painter.text(c.center(), egui::Align2::CENTER_CENTER, "🔥", FontId::proportional(14.0), Color32::WHITE);
                        }
                    }
                }
                ui.label(RichText::new(ts("🔥 цель выполнена · жёлтый — занимался · синий — заморозка")).size(12.0).color(p.muted));
                ui.add_space(16.0);
                ui.label(RichText::new(ts("Достижения")).size(18.0).strong().color(p.text));
                for row in gridcraft_learn_core::profile::ACHIEVEMENTS.chunks(7) {
                    ui.horizontal(|ui| {
                        for (id, icon, name, how) in row {
                            let got = prof.achievements.contains_key(*id);
                            let fill = if got { p.gold } else { p.grey };
                            egui::Frame::NONE
                                .fill(fill)
                                .corner_radius(12.0)
                                .inner_margin(egui::Margin::symmetric(10, 6))
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::Label::new(RichText::new(format!("{icon} {}", ts(name))).strong().color(if got {
                                            Color32::WHITE
                                        } else {
                                            p.muted
                                        }))
                                        .wrap_mode(egui::TextWrapMode::Extend),
                                    );
                                })
                                .response
                                .on_hover_text(ts(how));
                        }
                    });
                }
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(ts("🏁 Спринты")).size(18.0).strong().color(p.text));
                    if ui.button(ts("Спринт: микс")).clicked() {
                        sprint = Some(None);
                    }
                });
                let mut kinds: Vec<String> = prof.sprints.iter().map(|r| r.kind.clone()).collect();
                kinds.sort();
                kinds.dedup();
                if kinds.is_empty() {
                    ui.label(
                        RichText::new(ts("Пока нет заездов. Спринт — 10 заданий на время: копи рекорды, как в киберспорте по Excel.")).color(p.muted),
                    );
                }
                egui::Grid::new("sprints").striped(true).min_col_width(110.0).show(ui, |ui| {
                    if !kinds.is_empty() {
                        for h in [ts("Заезд"), ts("Рекорд"), ts("Последний"), ts("Заездов")] {
                            ui.label(RichText::new(h).strong());
                        }
                        ui.end_row();
                    }
                    for k in &kinds {
                        let name = if k == "mix" { ts("Микс") } else { SKILLS.iter().find(|s| s.id == k).map(|s| ts(s.name)).unwrap_or(k) };
                        let runs: Vec<_> = prof.sprints.iter().filter(|r| &r.kind == k).collect();
                        ui.label(name);
                        ui.label(prof.sprint_best(k).map(clock).unwrap_or_default());
                        ui.label(runs.last().map(|r| clock(r.seconds)).unwrap_or_default());
                        ui.label(runs.len().to_string());
                        ui.end_row();
                    }
                });
                ui.add_space(16.0);
                ui.label(RichText::new(ts("📈 Замер прогресса")).size(18.0).strong().color(p.text));
                ui.label(
                    RichText::new(ts("Одни и те же 8 навыков на новых данных: до обучения, после и через неделю. Рост «до → после» — навык появился, «после → через неделю» — удержался."))
                        .size(12.0)
                        .color(p.muted),
                );
                if prof.checkpoints.is_empty() {
                    ui.label(RichText::new(ts("Срезов пока нет — начни со среза «До обучения» на главной.")).color(p.muted));
                } else {
                    egui::Grid::new("checkpoints").striped(true).min_col_width(110.0).show(ui, |ui| {
                        for h in [ts("Срез"), ts("Когда"), ts("Решено"), ts("Время")] {
                            ui.label(RichText::new(h).strong());
                        }
                        ui.end_row();
                        let t = now();
                        for c in &prof.checkpoints {
                            let days = t.saturating_sub(c.at) / gridcraft_learn_core::profile::DAY;
                            ui.label(ts(c.phase.label()));
                            ui.label(if days == 0 { ts("сегодня").to_string() } else { tr(&format!("{days} дн. назад")) });
                            ui.label(format!("{}/{}", c.solved, c.total));
                            ui.label(clock(c.seconds));
                            ui.end_row();
                        }
                    });
                    let first = prof.checkpoints.first();
                    let last = prof.checkpoints.last();
                    if let (Some(a), Some(b)) = (first, last)
                        && prof.checkpoints.len() > 1
                    {
                        let d = b.solved as i64 - a.solved as i64;
                        let sign = if d > 0 { "+" } else { "" };
                        ui.label(RichText::new(tr(&format!("С первого среза: {sign}{d} задач", ))).strong().color(if d >= 0 { p.green_dark } else { p.red }));
                    }
                }
                if !prof.transfers.is_empty() {
                    let done = prof.transfers.iter().filter(|t| t.score >= 0.999).count();
                    ui.add_space(6.0);
                    ui.label(RichText::new(tr(&format!("Перенос в Excel: {done} из {} файлов сданы без замечаний", prof.transfers.len()))).color(p.text));
                }
                if prof.method_total > 0 {
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(tr(&format!("Выбор метода: {} из {} верно с первого раза", prof.method_right, prof.method_total))).color(p.text),
                    );
                }
                ui.add_space(16.0);
                ui.label(RichText::new(ts("Навыки")).size(18.0).strong().color(p.text));
                ui.label(
                    RichText::new(ts("Интервалы повторений — FSRS-4.5 с параметрами по умолчанию. 🏁 — спринт по навыку.")).size(12.0).color(p.muted),
                );
                for (u, unit) in UNITS.iter().enumerate().take(UNITS.len() - 1) {
                    ui.add_space(6.0);
                    ui.label(RichText::new(ts(unit.title)).size(16.0).strong().color(unit_color(u)));
                    egui::Grid::new(("stats", u)).striped(true).min_col_width(80.0).show(ui, |ui| {
                        for h in [ts("Навык"), ts("Уровень"), ts("Попыток"), ts("Верно"), ts("Лучшее время"), ts("Повтор"), ""]
                        {
                            ui.label(RichText::new(h).strong());
                        }
                        ui.end_row();
                        for s in SKILLS.iter().filter(|s| s.unit == u) {
                            let sp = prof.skills.get(s.id);
                            ui.label(ts(s.name));
                            ui.label(level_name(prof.level(s.id)));
                            ui.label(sp.map(|x| x.attempts.to_string()).unwrap_or_else(|| "—".into()));
                            ui.label(
                                sp.map(|x| format!("{:.0}%", x.successes as f32 * 100.0 / x.attempts.max(1) as f32)).unwrap_or_else(|| "—".into()),
                            );
                            ui.label(
                                sp.filter(|x| x.best_seconds > 0.0).map(|x| tr(&format!("{:.0} с", x.best_seconds))).unwrap_or_else(|| "—".into()),
                            );
                            ui.label(
                                sp.filter(|x| x.card.reps > 0)
                                    .map(|x| {
                                        let d = x.card.due.saturating_sub(t) as f32 / 86_400.0;
                                        if d <= 0.0 { ts("сегодня").into() } else { tr(&format!("через {d:.0} дн.")) }
                                    })
                                    .unwrap_or_else(|| "—".into()),
                            );
                            if prof.level(s.id) >= 2 {
                                if ui.small_button("🏁").on_hover_text(ts("Спринт по этому навыку")).clicked() {
                                    sprint = Some(Some(s.id.to_string()));
                                }
                            } else {
                                ui.label("");
                            }
                            ui.end_row();
                        }
                    });
                }
                ui.add_space(16.0);
                ui.label(RichText::new(ts("Экспресс-тесты")).size(18.0).strong().color(p.text));
                if prof.blitz.is_empty() {
                    ui.label(ts("Пока не было."));
                }
                for b in prof.blitz.iter().rev().take(10) {
                    ui.label(tr(&format!(
                        "{}: {}/{} верно, {} очков, {}",
                        b.track.map(|t| t.title()).unwrap_or(""),
                        b.correct,
                        b.total,
                        b.points,
                        clock(b.seconds)
                    )));
                }
                ui.add_space(12.0);
                ui.label(RichText::new(ts("Собеседования")).size(18.0).strong().color(p.text));
                if prof.interviews.is_empty() {
                    ui.label(ts("Пока не было."));
                }
                for iv in prof.interviews.iter().rev().take(20) {
                    let title = CASES.iter().find(|c| c.id == iv.case).map(|c| ts(c.title)).unwrap_or("");
                    ui.label(tr(&format!("{title}: {:.0}% за {:.0} из {} мин", iv.score * 100.0, (iv.seconds / 60.0).ceil(), iv.minutes)));
                }
            });
        });
    });
    if back {
        app.screen = Screen::Home;
    }
    if let Some(s) = sprint {
        let r = app.start_sprint(s.as_deref());
        app.handle(r);
    }
}

pub fn settings(app: &mut LearningApp, ctx: &egui::Context) {
    let mut open = app.show_settings;
    let mut remind: Option<Option<(u8, u8)>> = None;
    egui::Window::new(ts("Настройки")).open(&mut open).collapsible(false).resizable(false).show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label("Язык / Language:");
            let mut en = app.profile.english;
            ui.selectable_value(&mut en, false, "Русский");
            ui.selectable_value(&mut en, true, "English");
            if en != app.profile.english {
                app.profile.english = en;
                gridcraft_tr::set_english(en);
                app.sheet.ui.language = if en { gridcraft_ui_egui::i18n::Language::En } else { gridcraft_ui_egui::i18n::Language::Ru };
                app.greeting = Some(crate::remind::greeting(&app.profile, now()));
            }
        });
        ui.checkbox(&mut app.sheet.ui.dark, ts("Тёмная тема"));
        ui.checkbox(&mut app.profile.sound, ts("Звуки"));
        ui.horizontal(|ui| {
            ui.label(ts("Цель на день:"));
            for (xp, name) in [(30, ts("Лёгкая")), (50, ts("Обычная")), (100, ts("Серьёзная")), (200, ts("Интенсив"))] {
                if ui.selectable_label(app.profile.goal == xp, format!("{name} · {xp} XP")).clicked() {
                    app.profile.goal = xp;
                    app.profile.quests.clear();
                }
            }
        });
        ui.separator();
        ui.label(RichText::new(ts("🔔 Ежедневное напоминание")).strong());
        let mut on = app.profile.remind.is_some();
        let (mut h, m) = app.profile.remind.unwrap_or((19, 0));
        ui.horizontal(|ui| {
            if ui.checkbox(&mut on, ts("Напоминать, если цель дня не выполнена, в")).changed() {
                remind = Some(if on { Some((h, m)) } else { None });
            }
            egui::ComboBox::from_id_salt("remind_hour").selected_text(format!("{h:02}:00")).show_ui(ui, |ui| {
                for hour in 8..=23u8 {
                    if ui.selectable_value(&mut h, hour, format!("{hour:02}:00")).clicked() && on {
                        remind = Some(Some((hour, 0)));
                    }
                }
            });
        });
        ui.label(RichText::new(ts("Уведомление рабочего стола через systemd (таймер пользователя). Ничего не отправляется в сеть.")).size(12.0));
        if let Some(st) = &app.remind_status {
            ui.label(RichText::new(st).size(12.0));
        }
        ui.separator();
        if let Some(config) = &app.ai_config {
            ui.label(format!("AI: {} · {}", config.label(), config.model));
            ui.checkbox(&mut app.ai_consent, ts("Разрешить отправку условия, учебных данных и моих формул выбранной модели"));
            ui.label(ts("Запрос уходит только по кнопке «Подсказка AI». Лимит: 1 в минуту, 20 за запуск. AI не меняет таблицу и не ставит оценки."));
        } else {
            ui.label(ts("AI-подсказки выключены: всё работает локально. Как подключить Gemini/Ollama — в README-LEARN.md."));
        }
        ui.separator();
        ui.label(ts("Формулы вводятся по-русски, как в русском Excel: =СУММ(A1:A5), разделитель «;», дробная часть через запятую. Английские имена тоже работают."));
    });
    app.show_settings = open;
    if let Some(r) = remind {
        app.set_reminder(r);
    }
}
