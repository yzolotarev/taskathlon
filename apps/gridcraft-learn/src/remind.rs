//! Daily motivation: the mascot's greeting and an optional desktop reminder
//! (a systemd user timer that runs `taskathlon --remind`).

use gridcraft_learn_core::{Profile, Save, content::SKILLS};
use gridcraft_tr::{t as tr, ts};
use std::path::{Path, PathBuf};

/// One line from the mascot, chosen from the learner's state and the day.
pub fn greeting(p: &Profile, now: u64) -> String {
    let streak = p.current_streak(now);
    let today = p.today_xp(now);
    let day = p.day(now) as usize;
    let due = p.due(now).len();
    if p.log.is_empty() {
        return ts("Привет! Я Сэл. Первый урок — пять минут. Жми Enter!").into();
    }
    if p.streak_at_risk(now) {
        return tr(&format!("🔥 Серия {streak} дн. под угрозой! Один урок — и она спасена."));
    }
    if let Some(w) = &p.wager {
        let left = (w.start_day + u64::from(w.days)).saturating_sub(p.day(now));
        if left > 0 {
            return tr(&format!("🎲 Ставка в силе: ещё {left} дн. — и получишь {} 💎.", w.stake * 2));
        }
    }
    if today >= p.goal {
        let lines = [
            ts("Цель дня есть! Ещё один урок — и завтра будет легче."),
            ts("Норма сделана. Хочешь — пробегись по экспресс-тесту на скорость."),
            ts("Красота! Можно потренировать собеседование, пока голова свежая."),
        ];
        return lines.get(day % lines.len()).copied().unwrap_or("").into();
    }
    if due >= 2 {
        return tr(&format!("{due} навыков пора повторить, пока не забылись. Это быстро."));
    }
    if today > 0 {
        return tr(&format!("Ещё {} XP до цели дня. Ты почти у цели!", p.goal - today));
    }
    let next = p.frontier().and_then(|f| SKILLS.iter().find(|s| s.id == f)).map(|s| ts(s.name)).unwrap_or(ts("повторение"));
    let lines = [
        tr(&format!("Сегодня по плану: «{next}». Пять минут — и вперёд.")),
        ts("Каждый день понемногу — и на собеседовании руки делают сами.").to_string(),
        tr(&format!("Серия {streak} дн. Продолжим?")),
        ts("Один урок сейчас лучше трёх «потом».").to_string(),
    ];
    lines.get(day % lines.len()).cloned().unwrap_or_default()
}

/// What the evening reminder says, or `None` when today's goal is already met.
pub fn reminder_text(p: &Profile, now: u64) -> Option<(String, String)> {
    if p.today_xp(now) >= p.goal {
        return None;
    }
    let streak = p.current_streak(now);
    let title = if p.streak_at_risk(now) {
        tr(&format!("🔥 Серия {streak} дн. сгорит в полночь"))
    } else {
        ts("Сэл ждёт тебя на урок").to_string()
    };
    let body = if p.today_xp(now) > 0 {
        tr(&format!("До цели дня осталось {} XP. Открой Taskathlon и нажми Enter.", p.goal - p.today_xp(now)))
    } else {
        ts("Пять минут практики — открой Taskathlon и нажми Enter.").into()
    };
    Some((title, body))
}

/// `taskathlon --remind`: one desktop notification if today's goal is not met.
pub fn run(state: &Path, now: u64) -> Result<(), String> {
    let bytes = std::fs::read(state).map_err(|e| tr(&format!("Нет файла прогресса {}: {e}", state.display())))?;
    let save = Save::decode(&bytes).map_err(|e| e.to_string())?;
    let Some((title, body)) = reminder_text(&save.profile, now) else { return Ok(()) };
    std::process::Command::new("notify-send")
        .args(["-a", "Taskathlon", "-u", "normal", &title, &body])
        .status()
        .map_err(|e| tr(&format!("notify-send недоступен: {e}")))?;
    Ok(())
}

fn unit_dir() -> Option<PathBuf> {
    let base =
        std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("systemd/user"))
}

const NAME: &str = "taskathlon-remind";

/// Installs (or with `None` removes) a daily systemd user timer at `at` local time.
pub fn install(at: Option<(u8, u8)>) -> Result<String, String> {
    let dir = unit_dir().ok_or(ts("Не найден домашний каталог"))?;
    let service = dir.join(format!("{NAME}.service"));
    let timer = dir.join(format!("{NAME}.timer"));
    let systemctl = |args: &[&str]| std::process::Command::new("systemctl").arg("--user").args(args).status().map(|s| s.success()).unwrap_or(false);
    let Some((h, m)) = at else {
        systemctl(&["disable", "--now", &format!("{NAME}.timer")]);
        let _ = std::fs::remove_file(&service);
        let _ = std::fs::remove_file(&timer);
        return Ok(ts("Напоминание выключено.").into());
    };
    let exe = std::env::current_exe().map_err(|e| tr(&format!("Не найден путь к программе: {e}")))?;
    let state = std::env::var("GRIDCRAFT_LEARN_STATE").map(|s| format!("Environment=GRIDCRAFT_LEARN_STATE={s}\n")).unwrap_or_default();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(
        &service,
        format!(
            "[Unit]\nDescription=Taskathlon: напоминание о занятии\n\n[Service]\nType=oneshot\n{state}ExecStart=\"{}\" --remind\n",
            exe.display()
        ),
    )
    .map_err(|e| e.to_string())?;
    std::fs::write(&timer, format!("[Unit]\nDescription=Taskathlon: ежедневное напоминание\n\n[Timer]\nOnCalendar=*-*-* {h:02}:{m:02}:00\nPersistent=true\n\n[Install]\nWantedBy=timers.target\n"))
        .map_err(|e| e.to_string())?;
    if systemctl(&["daemon-reload"]) && systemctl(&["enable", "--now", &format!("{NAME}.timer")]) {
        Ok(tr(&format!("Напомню каждый день в {h:02}:{m:02}, если цель дня ещё не выполнена.")))
    } else {
        Err(format!(
            "Файлы таймера записаны в {}, но systemd не ответил. Можно добавить в crontab:\n{m} {h} * * * \"{}\" --remind",
            dir.display(),
            exe.display()
        ))
    }
}
