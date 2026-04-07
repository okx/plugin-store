use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use plugin_store::registry::RegistryManager;
use plugin_store::stats::StatsMap;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState, Tabs, Wrap,
    },
    Frame, Terminal,
};
use plugin_store::registry::models::Plugin;
use plugin_store::stats;

#[derive(Clone)]
enum View {
    List,
    Detail(usize), // index into current tab's plugin list
}

struct App {
    tabs: Vec<String>,
    active_tab: usize,
    table_states: Vec<TableState>,
    plugins_by_tab: Vec<Vec<Plugin>>,
    view: View,
}

impl App {
    fn new(all_plugins: Vec<Plugin>, counts: &StatsMap) -> Self {
        let mut all_sorted = all_plugins.clone();
        all_sorted.sort_by(|a, b| {
            let da = counts.get(&a.name).copied().unwrap_or(0);
            let db = counts.get(&b.name).copied().unwrap_or(0);
            db.cmp(&da)
        });

        let strategies: Vec<Plugin> = all_sorted
            .iter()
            .filter(|p| p.category == "trading-strategy")
            .cloned()
            .collect();

        let dapps: Vec<Plugin> = all_sorted
            .iter()
            .filter(|p| p.category == "defi-protocol")
            .cloned()
            .collect();

        let plugins_by_tab = vec![all_sorted, strategies, dapps];
        let tabs_count = 3;

        let table_states: Vec<TableState> = (0..tabs_count)
            .map(|i| {
                let mut s = TableState::default();
                if !plugins_by_tab[i].is_empty() {
                    s.select(Some(0));
                }
                s
            })
            .collect();

        App {
            tabs: vec!["推荐".to_string(), "策略".to_string(), "DApp".to_string()],
            active_tab: 0,
            table_states,
            plugins_by_tab,
            view: View::List,
        }
    }

    fn selected_idx(&self) -> Option<usize> {
        self.table_states[self.active_tab].selected()
    }

    fn select_next(&mut self) {
        let len = self.plugins_by_tab[self.active_tab].len();
        if len == 0 {
            return;
        }
        let i = match self.table_states[self.active_tab].selected() {
            Some(i) => (i + 1).min(len - 1),
            None => 0,
        };
        self.table_states[self.active_tab].select(Some(i));
    }

    fn select_prev(&mut self) {
        let len = self.plugins_by_tab[self.active_tab].len();
        if len == 0 {
            return;
        }
        let i = match self.table_states[self.active_tab].selected() {
            Some(0) | None => 0,
            Some(i) => i - 1,
        };
        self.table_states[self.active_tab].select(Some(i));
    }
}

pub async fn execute() -> Result<()> {
    use std::io::IsTerminal;

    let manager = RegistryManager::new();
    let registry = manager.get_registry(false).await?;
    let counts = stats::fetch(registry.stats_url.as_deref()).await;

    if registry.plugins.is_empty() {
        println!("No plugins available.");
        return Ok(());
    }

    // Non-TTY (agent/pipe): plain text output, sorted by downloads desc
    if !std::io::stdout().is_terminal() {
        let mut plugins = registry.plugins.clone();
        plugins.sort_by(|a, b| {
            let da = counts.get(&a.name).copied().unwrap_or(0);
            let db = counts.get(&b.name).copied().unwrap_or(0);
            db.cmp(&da)
        });
        return execute_plain(&plugins, &counts);
    }

    enable_raw_mode()?;
    // Ensure terminal is restored even on panic or early error
    struct TerminalGuard;
    impl Drop for TerminalGuard {
        fn drop(&mut self) {
            let _ = disable_raw_mode();
            let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
        }
    }
    let _guard = TerminalGuard;

    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(registry.plugins, &counts);
    let mut install_name: Option<String> = None;

    'tui: loop {
        terminal.draw(|f| render(f, &mut app, &counts))?;

        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match &app.view {
                    View::Detail(idx) => match key.code {
                        KeyCode::Char('i') => {
                            let plugins = &app.plugins_by_tab[app.active_tab];
                            if let Some(p) = plugins.get(*idx) {
                                install_name = Some(p.name.clone());
                                break 'tui;
                            }
                        }
                        _ => {
                            app.view = View::List;
                        }
                    },
                    View::List => match (key.code, key.modifiers) {
                        (KeyCode::Char('q'), _)
                        | (KeyCode::Esc, _)
                        | (KeyCode::Char('c'), KeyModifiers::CONTROL) => break 'tui,
                        (KeyCode::Left, _) => {
                            if app.active_tab > 0 {
                                app.active_tab -= 1;
                            }
                        }
                        (KeyCode::Right, _) => {
                            if app.active_tab < app.tabs.len() - 1 {
                                app.active_tab += 1;
                            }
                        }
                        (KeyCode::Tab, _) => {
                            app.active_tab = (app.active_tab + 1) % app.tabs.len();
                        }
                        (KeyCode::Down, _) | (KeyCode::Char('j'), _) => app.select_next(),
                        (KeyCode::Up, _) | (KeyCode::Char('k'), _) => app.select_prev(),
                        (KeyCode::Enter, _) => {
                            if let Some(idx) = app.selected_idx() {
                                app.view = View::Detail(idx);
                            }
                        }
                        _ => {}
                    },
                }
            }
        }
    }

    // TerminalGuard drop handles raw_mode + alternate screen restore
    terminal.show_cursor()?;
    drop(_guard); // restore before printing install output

    if let Some(name) = install_name {
        super::install::execute(&name, false, false, None, false).await?;
    }

    Ok(())
}

fn execute_plain(plugins: &[Plugin], counts: &StatsMap) -> Result<()> {
    println!("{:<40} {:<10} {:<10} {}", "Name", "Version", "Downloads", "Description");
    println!("{}", "-".repeat(100));
    for p in plugins {
        let downloads = counts.get(&p.name).copied().unwrap_or(0);
        let dl = if downloads == 0 { "-".to_string() } else { format_downloads(downloads) };
        println!("{:<40} {:<10} {:<10} {}", p.name, p.version, dl, p.description);
    }
    println!("\n{} plugins available. Use `npx skills add MigOKG/plugin-store --name <name>` to install.", plugins.len());
    Ok(())
}

fn render(f: &mut Frame, app: &mut App, counts: &StatsMap) {
    let size = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(size);

    render_tabs(f, app, chunks[0]);
    render_table(f, app, counts, chunks[1]);

    let detail_idx = match app.view {
        View::Detail(i) => Some(i),
        _ => None,
    };
    render_help(f, chunks[2], detail_idx.is_some());

    if let Some(idx) = detail_idx {
        let plugins = &app.plugins_by_tab[app.active_tab];
        if let Some(plugin) = plugins.get(idx) {
            let count = counts.get(&plugin.name).copied().unwrap_or(0);
            render_detail_popup(f, plugin, count, size);
        }
    }
}

fn render_tabs(f: &mut Frame, app: &App, area: Rect) {
    let tab_titles: Vec<Line> = app
        .tabs
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let count = app.plugins_by_tab[i].len();
            Line::from(Span::raw(format!(" {} ({}) ", t, count)))
        })
        .collect();

    let tabs = Tabs::new(tab_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Plugin Store ")
                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        )
        .select(app.active_tab)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(tabs, area);
}

fn render_table(f: &mut Frame, app: &mut App, counts: &StatsMap, area: Rect) {
    let active_tab = app.active_tab;
    let plugins = app.plugins_by_tab[active_tab].clone();

    let header_cells = ["#", "Name", "Version", "Downloads", "Source", "Description"]
        .iter()
        .map(|h| {
            Cell::from(*h)
                .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        });
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows: Vec<Row> = plugins
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let downloads = counts.get(&p.name).copied().unwrap_or(0);
            let downloads_str = if downloads == 0 {
                Span::styled("-", Style::default().fg(Color::DarkGray))
            } else {
                Span::styled(
                    format_downloads(downloads),
                    Style::default().fg(Color::Green),
                )
            };

            let source_span = match p.source.as_str() {
                "official" => {
                    Span::styled(p.source.clone(), Style::default().fg(Color::Green))
                }
                "community" => {
                    Span::styled(p.source.clone(), Style::default().fg(Color::Yellow))
                }
                _ => Span::styled(p.source.clone(), Style::default().fg(Color::Cyan)),
            };

            let name_cell = Cell::from(Span::styled(p.name.clone(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)));

            Row::new(vec![
                Cell::from(Span::styled(
                    format!("{:>2}", i + 1),
                    Style::default().fg(Color::DarkGray),
                )),
                name_cell,
                Cell::from(Span::styled(
                    p.version.clone(),
                    Style::default().fg(Color::Blue),
                )),
                Cell::from(downloads_str),
                Cell::from(source_span),
                Cell::from(Span::styled(
                    p.description.clone(),
                    Style::default().fg(Color::Gray),
                )),
            ])
            .height(1)
        })
        .collect();

    let total = plugins.len();
    let selected = app.table_states[active_tab].selected().map(|i| i + 1).unwrap_or(0);
    let title = if total == 0 {
        " No plugins ".to_string()
    } else {
        format!(" {}/{} ", selected, total)
    };

    let table = Table::new(
        rows,
        [
            Constraint::Length(3),
            Constraint::Min(40),
            Constraint::Length(9),
            Constraint::Length(11),
            Constraint::Length(12),
            Constraint::Min(20),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .title_style(Style::default().fg(Color::DarkGray)),
    )
    .row_highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    );

    f.render_stateful_widget(table, area, &mut app.table_states[active_tab]);
}

fn render_help(f: &mut Frame, area: Rect, in_detail: bool) {
    let spans = if in_detail {
        vec![
            Span::styled("i", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" install  "),
            Span::styled("any other key", Style::default().fg(Color::Yellow)),
            Span::raw(" close"),
        ]
    } else {
        vec![
            Span::styled("←/→ Tab", Style::default().fg(Color::Yellow)),
            Span::raw(" switch  "),
            Span::styled("↑/↓ j/k", Style::default().fg(Color::Yellow)),
            Span::raw(" move  "),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(" detail  "),
            Span::styled("q/Esc", Style::default().fg(Color::Yellow)),
            Span::raw(" quit"),
        ]
    };

    let help = Paragraph::new(Line::from(spans)).style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, area);
}

fn render_detail_popup(f: &mut Frame, plugin: &Plugin, downloads: u64, area: Rect) {
    // Centered popup: 70% width, 60% height
    let popup_area = centered_rect(70, 70, area);

    // Clear background
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", plugin.name))
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    // Build content lines
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(vec![
        Span::styled("Description: ", Style::default().fg(Color::Yellow)),
        Span::raw(plugin.description.clone()),
    ]));
    lines.push(Line::from(""));

    lines.push(Line::from(vec![
        Span::styled("Version:     ", Style::default().fg(Color::Yellow)),
        Span::styled(plugin.version.clone(), Style::default().fg(Color::Blue)),
        Span::raw("   "),
        Span::styled("Downloads: ", Style::default().fg(Color::Yellow)),
        Span::styled(
            if downloads == 0 {
                "-".to_string()
            } else {
                format_downloads(downloads)
            },
            Style::default().fg(Color::Green),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Category:    ", Style::default().fg(Color::Yellow)),
        Span::raw(plugin.category.clone()),
        Span::raw("   "),
        Span::styled("Source: ", Style::default().fg(Color::Yellow)),
        Span::raw(plugin.source.clone()),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Author:      ", Style::default().fg(Color::Yellow)),
        Span::raw(match &plugin.link {
            Some(link) => format!("{} ({})", plugin.author.name, link),
            None => plugin.author.name.clone(),
        }),
    ]));

    if !plugin.tags.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Tags:        ", Style::default().fg(Color::Yellow)),
            Span::styled(plugin.tags.join(", "), Style::default().fg(Color::Cyan)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Components",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )));

    if let Some(skill) = &plugin.components.skill {
        lines.push(Line::from(vec![
            Span::styled("  Skill  ", Style::default().fg(Color::Green)),
            Span::raw(format!("  repo: {}", skill.repo)),
        ]));
    }
    if let Some(mcp) = &plugin.components.mcp {
        lines.push(Line::from(vec![
            Span::styled("  MCP    ", Style::default().fg(Color::Magenta)),
            Span::raw(format!("  {} {}", mcp.command, mcp.args.join(" "))),
        ]));
    }
    if let Some(bin) = &plugin.components.binary {
        lines.push(Line::from(vec![
            Span::styled("  Binary ", Style::default().fg(Color::Blue)),
            Span::raw(format!("  repo: {}", bin.repo)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Install: ", Style::default().fg(Color::Yellow)),
        Span::styled(
            format!("plugin-store install {}", plugin.name),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(
        Span::styled(
            "  [ i ] Install Now  ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
    ));

    let para = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .alignment(Alignment::Left);

    f.render_widget(para, inner);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn format_downloads(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
