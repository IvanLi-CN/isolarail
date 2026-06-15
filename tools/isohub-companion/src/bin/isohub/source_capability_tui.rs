#[derive(Clone, Copy)]
enum SourceCapabilityEditorRow {
    PowerWatts,
    Pd,
    Pps,
    Qc20,
    Qc30,
    Fcp,
    Afc,
    Scp,
    Pe20,
    Bc12,
    Sfcp,
    FixedPd,
    Pps3Limit,
    PdPps5a,
    TypeCBroadcast,
    ScpLimit,
    FcpAfcSfcpLimit,
    Actions,
}

const SOURCE_CAPABILITY_EDITOR_ROWS: [SourceCapabilityEditorRow; 18] = [
    SourceCapabilityEditorRow::PowerWatts,
    SourceCapabilityEditorRow::Pd,
    SourceCapabilityEditorRow::Pps,
    SourceCapabilityEditorRow::Qc20,
    SourceCapabilityEditorRow::Qc30,
    SourceCapabilityEditorRow::Fcp,
    SourceCapabilityEditorRow::Afc,
    SourceCapabilityEditorRow::Scp,
    SourceCapabilityEditorRow::Pe20,
    SourceCapabilityEditorRow::Bc12,
    SourceCapabilityEditorRow::Sfcp,
    SourceCapabilityEditorRow::FixedPd,
    SourceCapabilityEditorRow::Pps3Limit,
    SourceCapabilityEditorRow::PdPps5a,
    SourceCapabilityEditorRow::TypeCBroadcast,
    SourceCapabilityEditorRow::ScpLimit,
    SourceCapabilityEditorRow::FcpAfcSfcpLimit,
    SourceCapabilityEditorRow::Actions,
];

struct SourceCapabilityEditorState {
    selected_row: usize,
    fixed_pd_focus: usize,
    action_focus: usize,
}

impl Default for SourceCapabilityEditorState {
    fn default() -> Self {
        Self {
            selected_row: 0,
            fixed_pd_focus: 0,
            action_focus: 0,
        }
    }
}

fn power_watt_choices(current: u8) -> Vec<u8> {
    let mut choices = POWER_WATT_PRESETS.to_vec();
    if !choices.contains(&current) {
        choices.push(current);
        choices.sort_unstable();
    }
    choices
}

fn cycle_choice<T: Copy + PartialEq>(current: T, choices: &[T], direction: i8) -> T {
    let len = choices.len();
    if len == 0 {
        return current;
    }
    let current_index = choices
        .iter()
        .position(|choice| *choice == current)
        .unwrap_or(0);
    let next_index = match direction.cmp(&0) {
        std::cmp::Ordering::Less => current_index.checked_sub(1).unwrap_or(len - 1),
        std::cmp::Ordering::Equal => current_index,
        std::cmp::Ordering::Greater => (current_index + 1) % len,
    };
    choices[next_index]
}

fn cycle_index(current: usize, len: usize, direction: i8) -> usize {
    if len == 0 {
        return current;
    }
    match direction.cmp(&0) {
        std::cmp::Ordering::Less => current.checked_sub(1).unwrap_or(len - 1),
        std::cmp::Ordering::Equal => current,
        std::cmp::Ordering::Greater => (current + 1) % len,
    }
}

fn with_tui_terminal<T>(
    viewport_height: u16,
    run: impl FnOnce(&mut DefaultTerminal) -> anyhow::Result<T>,
) -> anyhow::Result<T> {
    let viewport_height = viewport_height.max(3);
    let (width, height) =
        terminal::size().context("failed to read terminal size for compact TUI viewport")?;
    let viewport_height = viewport_height.min(height.max(1));
    let mut terminal = ratatui::try_init_with_options(TerminalOptions {
        viewport: Viewport::Fixed(Rect::new(
            0,
            height.saturating_sub(viewport_height),
            width,
            viewport_height,
        )),
    })
    .context("failed to initialize compact TUI viewport")?;
    let result = run(&mut terminal);
    ratatui::restore();
    result
}

fn anchored_panel_area(area: Rect, desired_width: u16, desired_height: u16) -> Rect {
    let width = desired_width.min(area.width).max(1);
    let height = desired_height.min(area.height).max(1);
    Rect::new(
        area.x,
        area.y + area.height.saturating_sub(height),
        width,
        height,
    )
}

fn panel_block(title: &str) -> Block<'static> {
    Block::bordered()
        .title(Span::styled(
            title.to_string(),
            Style::default().add_modifier(Modifier::BOLD),
        ))
        .border_style(Style::default().fg(Color::Gray))
        .style(Style::default().fg(Color::White))
}

fn text_width(text: &str) -> u16 {
    text.lines()
        .map(|line| line.chars().count() as u16)
        .max()
        .unwrap_or(0)
}

fn truncate_to_width(text: &str, max_width: u16) -> String {
    let max_width = max_width as usize;
    let char_count = text.chars().count();
    if char_count <= max_width {
        return text.to_string();
    }
    if max_width <= 1 {
        return "…".to_string();
    }
    let kept = max_width.saturating_sub(1);
    let mut truncated = text.chars().take(kept).collect::<String>();
    truncated.push('…');
    truncated
}

fn clamp_popup_width(area: Rect, desired_width: u16, minimum_width: u16) -> u16 {
    let available = area.width.max(1);
    desired_width
        .min(available)
        .max(minimum_width.min(available))
}

fn list_menu_viewport_height(subtitle: Option<&str>, items: &[String], footer: &[&str]) -> u16 {
    let subtitle_lines = subtitle
        .map(|text| text.lines().count() as u16)
        .unwrap_or(0);
    let footer_lines = footer.len() as u16;
    let item_lines = items.len().max(1) as u16;
    subtitle_lines + footer_lines + item_lines + 6
}

fn source_capability_viewport_height(diagnostics: &str) -> u16 {
    let status_lines = diagnostics.lines().count().min(6) as u16;
    let editor_lines = SOURCE_CAPABILITY_EDITOR_ROWS.len() as u16;
    status_lines + editor_lines + 7
}

fn truncate_lines(text: &str, max_lines: usize) -> String {
    let mut lines = text.lines();
    let mut kept = Vec::new();
    for _ in 0..max_lines {
        let Some(line) = lines.next() else {
            break;
        };
        kept.push(line.to_string());
    }
    if lines.next().is_some() {
        kept.push("...".to_string());
    }
    kept.join("\n")
}

fn draw_tui_list_menu(
    frame: &mut Frame<'_>,
    title: &str,
    subtitle: Option<&str>,
    items: &[String],
    footer: &[&str],
    selected: usize,
) {
    let subtitle_height = subtitle
        .map(|text| text.lines().count().max(1) as u16)
        .unwrap_or(0);
    let footer_height = footer.len() as u16;
    let popup_height = subtitle_height + items.len().max(1) as u16 + footer_height + 4;
    let desired_width = text_width(title)
        .max(subtitle.map(text_width).unwrap_or(0))
        .max(items.iter().map(|item| text_width(item)).max().unwrap_or(0))
        .max(
            footer
                .iter()
                .map(|line| text_width(line))
                .max()
                .unwrap_or(0),
        )
        + 8;
    let popup = anchored_panel_area(
        frame.area(),
        clamp_popup_width(frame.area(), desired_width, 48),
        popup_height,
    );
    frame.render_widget(Clear, popup);
    let outer = panel_block(title);
    let inner = outer.inner(popup);
    frame.render_widget(outer, popup);

    let footer_height = if footer.is_empty() {
        0
    } else {
        footer.len() as u16
    };
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(subtitle_height),
            Constraint::Min(1),
            Constraint::Length(footer_height),
        ])
        .split(inner);

    if let Some(subtitle) = subtitle {
        frame.render_widget(
            Paragraph::new(subtitle)
                .style(Style::default().fg(Color::DarkGray))
                .wrap(Wrap { trim: false }),
            sections[0],
        );
    }

    let list_items = items
        .iter()
        .map(|item| {
            let fitted = truncate_to_width(item, sections[1].width.saturating_sub(4));
            ListItem::new(Line::from(Span::styled(
                fitted,
                Style::default().fg(Color::White),
            )))
        })
        .collect::<Vec<_>>();
    let list = List::new(list_items)
        .highlight_symbol("› ")
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .add_modifier(Modifier::BOLD),
        )
        .repeat_highlight_symbol(true);
    let mut state = ListState::default();
    state.select(Some(selected.min(items.len().saturating_sub(1))));
    frame.render_stateful_widget(list, sections[1], &mut state);

    if !footer.is_empty() {
        let footer_text = Text::from(
            footer
                .iter()
                .map(|line| {
                    Line::from(Span::styled(
                        (*line).to_string(),
                        Style::default().fg(Color::DarkGray),
                    ))
                })
                .collect::<Vec<_>>(),
        );
        frame.render_widget(Paragraph::new(footer_text), sections[2]);
    }
}

fn run_tui_list_menu(
    title: &str,
    subtitle: Option<&str>,
    items: &[String],
    _footer: &[&str],
) -> anyhow::Result<Option<usize>> {
    if items.is_empty() {
        return Ok(None);
    }
    if let Some(subtitle) = subtitle {
        println!("{subtitle}");
    }
    Ok(Select::new()
        .with_prompt(title)
        .items(items)
        .default(0)
        .interact_opt()?)
}

fn field_label(label: &str, selected: bool) -> Span<'static> {
    Span::styled(
        format!("{label}: "),
        if selected {
            Style::default().add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        },
    )
}

fn choice_chip(label: impl Into<String>, active: bool, focused: bool) -> Span<'static> {
    let label = label.into();
    let text = if active {
        format!("[{label}]")
    } else {
        format!(" {label} ")
    };
    let style = match (active, focused) {
        (true, true) => Style::default()
            .add_modifier(Modifier::REVERSED)
            .add_modifier(Modifier::BOLD),
        (false, true) => Style::default().add_modifier(Modifier::REVERSED),
        (true, false) => Style::default().add_modifier(Modifier::BOLD),
        (false, false) => Style::default().fg(Color::DarkGray),
    };
    Span::styled(text, style)
}

fn append_choice(
    spans: &mut Vec<Span<'static>>,
    label: impl Into<String>,
    active: bool,
    focused: bool,
) {
    spans.push(choice_chip(label, active, focused));
    spans.push(Span::raw(" "));
}

fn make_choice_row(
    label: &str,
    choices: impl IntoIterator<Item = (String, bool, bool)>,
    selected: bool,
) -> ListItem<'static> {
    let mut spans = vec![field_label(label, selected)];
    for (choice_label, active, focused) in choices {
        append_choice(&mut spans, choice_label, active, focused);
    }
    if spans
        .last()
        .is_some_and(|span| span.content.as_ref() == " ")
    {
        spans.pop();
    }
    ListItem::new(Line::from(spans))
}

fn render_source_capability_row(
    config: &CliPowerConfig,
    state: &SourceCapabilityEditorState,
    row_index: usize,
) -> ListItem<'static> {
    let row = SOURCE_CAPABILITY_EDITOR_ROWS[row_index];
    let selected = state.selected_row == row_index;
    match row {
        SourceCapabilityEditorRow::PowerWatts => {
            let current = config.capability.power_watts;
            let choices = power_watt_choices(current).into_iter().map(|value| {
                (
                    format!("{value} W"),
                    value == current,
                    selected && value == current,
                )
            });
            make_choice_row("Power cap", choices, selected)
        }
        SourceCapabilityEditorRow::Pd => make_choice_row(
            "PD",
            [false, true].into_iter().map(|value| {
                let active = protocol_enabled(&config.capability.protocols, "pd") == value;
                (on_off(value).to_string(), active, selected && active)
            }),
            selected,
        ),
        SourceCapabilityEditorRow::Pps => make_choice_row(
            "PPS",
            [false, true].into_iter().map(|value| {
                let active = config.capability.pd.pps == value;
                (on_off(value).to_string(), active, selected && active)
            }),
            selected,
        ),
        SourceCapabilityEditorRow::Qc20 => make_choice_row(
            "QC2.0",
            [false, true].into_iter().map(|value| {
                let active = protocol_enabled(&config.capability.protocols, "qc20") == value;
                (on_off(value).to_string(), active, selected && active)
            }),
            selected,
        ),
        SourceCapabilityEditorRow::Qc30 => make_choice_row(
            "QC3.0",
            [false, true].into_iter().map(|value| {
                let active = protocol_enabled(&config.capability.protocols, "qc30") == value;
                (on_off(value).to_string(), active, selected && active)
            }),
            selected,
        ),
        SourceCapabilityEditorRow::Fcp => make_choice_row(
            "FCP",
            [false, true].into_iter().map(|value| {
                let active = protocol_enabled(&config.capability.protocols, "fcp") == value;
                (on_off(value).to_string(), active, selected && active)
            }),
            selected,
        ),
        SourceCapabilityEditorRow::Afc => make_choice_row(
            "AFC",
            [false, true].into_iter().map(|value| {
                let active = protocol_enabled(&config.capability.protocols, "afc") == value;
                (on_off(value).to_string(), active, selected && active)
            }),
            selected,
        ),
        SourceCapabilityEditorRow::Scp => make_choice_row(
            "SCP",
            [false, true].into_iter().map(|value| {
                let active = protocol_enabled(&config.capability.protocols, "scp") == value;
                (on_off(value).to_string(), active, selected && active)
            }),
            selected,
        ),
        SourceCapabilityEditorRow::Pe20 => make_choice_row(
            "PE2.0",
            [false, true].into_iter().map(|value| {
                let active = protocol_enabled(&config.capability.protocols, "pe20") == value;
                (on_off(value).to_string(), active, selected && active)
            }),
            selected,
        ),
        SourceCapabilityEditorRow::Bc12 => make_choice_row(
            "BC1.2",
            [false, true].into_iter().map(|value| {
                let active = protocol_enabled(&config.capability.protocols, "bc12") == value;
                (on_off(value).to_string(), active, selected && active)
            }),
            selected,
        ),
        SourceCapabilityEditorRow::Sfcp => make_choice_row(
            "SFCP",
            [false, true].into_iter().map(|value| {
                let active = protocol_enabled(&config.capability.protocols, "sfcp") == value;
                (on_off(value).to_string(), active, selected && active)
            }),
            selected,
        ),
        SourceCapabilityEditorRow::FixedPd => make_choice_row(
            "Fixed PD",
            FIXED_PD_OPTIONS.iter().enumerate().map(|(index, value)| {
                (
                    format!("{}V", value / 1000),
                    config.capability.pd.fixed_voltages_mv.contains(value),
                    selected && state.fixed_pd_focus == index,
                )
            }),
            selected,
        ),
        SourceCapabilityEditorRow::Pps3Limit => make_choice_row(
            "PPS3 limit",
            [3000_u16, 5000].into_iter().map(|value| {
                let active = config.capability.current.pps3_limit_ma == value;
                (format!("{value} mA"), active, selected && active)
            }),
            selected,
        ),
        SourceCapabilityEditorRow::PdPps5a => make_choice_row(
            "PD/PPS 5 A",
            [false, true].into_iter().map(|value| {
                let active = config.capability.current.pd_pps_5a == value;
                (on_off(value).to_string(), active, selected && active)
            }),
            selected,
        ),
        SourceCapabilityEditorRow::TypeCBroadcast => make_choice_row(
            "Type-C current",
            [500_u16, 1500].into_iter().map(|value| {
                let active = config.capability.current.type_c_broadcast_ma == value;
                (format!("{value} mA"), active, selected && active)
            }),
            selected,
        ),
        SourceCapabilityEditorRow::ScpLimit => make_choice_row(
            "SCP current",
            [2000_u16, 4000, 5000].into_iter().map(|value| {
                let active = config.capability.current.scp_limit_ma == value;
                (format!("{value} mA"), active, selected && active)
            }),
            selected,
        ),
        SourceCapabilityEditorRow::FcpAfcSfcpLimit => make_choice_row(
            "FCP/AFC/SFCP current",
            [2250_u16, 3250].into_iter().map(|value| {
                let active = config.capability.current.fcp_afc_sfcp_limit_ma == value;
                (format!("{value} mA"), active, selected && active)
            }),
            selected,
        ),
        SourceCapabilityEditorRow::Actions => make_choice_row(
            "Action",
            ACTION_OPTIONS.iter().enumerate().map(|(index, label)| {
                (
                    (*label).to_string(),
                    false,
                    selected && state.action_focus == index,
                )
            }),
            selected,
        ),
    }
}

fn draw_source_capability_editor(
    frame: &mut Frame<'_>,
    diagnostics: &str,
    config: &CliPowerConfig,
    state: &SourceCapabilityEditorState,
) {
    let diagnostics = truncate_lines(diagnostics, 6);
    let status_lines = diagnostics.lines().count().max(1) as u16;
    let footer_lines = 2_u16;
    let popup_height = status_lines + SOURCE_CAPABILITY_EDITOR_ROWS.len() as u16 + footer_lines + 4;
    let popup = anchored_panel_area(
        frame.area(),
        clamp_popup_width(frame.area(), 104, 72),
        popup_height,
    );
    frame.render_widget(Clear, popup);
    let outer = panel_block("Source capability editor");
    let inner = outer.inner(popup);
    frame.render_widget(outer, popup);

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(status_lines),
            Constraint::Min(10),
            Constraint::Length(footer_lines),
        ])
        .split(inner);

    frame.render_widget(
        Paragraph::new(diagnostics)
            .style(Style::default().fg(Color::DarkGray))
            .wrap(Wrap { trim: false }),
        sections[0],
    );

    let items = SOURCE_CAPABILITY_EDITOR_ROWS
        .iter()
        .enumerate()
        .map(|(index, _)| render_source_capability_row(config, state, index))
        .collect::<Vec<_>>();
    let list = List::new(items).highlight_symbol("› ");
    let mut list_state = ListState::default();
    list_state.select(Some(state.selected_row));
    frame.render_stateful_widget(list, sections[1], &mut list_state);

    let footer = Text::from(vec![
        Line::from("Use Up/Down to choose a field. Use Left/Right to change the current field."),
        Line::from(
            "Press Enter/Space to toggle the focused chip. Use the Action row to save, reload, or cancel.",
        ),
    ]);
    frame.render_widget(
        Paragraph::new(footer).style(Style::default().fg(Color::DarkGray)),
        sections[2],
    );
}

fn apply_row_direction(
    config: &mut CliPowerConfig,
    state: &mut SourceCapabilityEditorState,
    direction: i8,
) -> anyhow::Result<()> {
    match SOURCE_CAPABILITY_EDITOR_ROWS[state.selected_row] {
        SourceCapabilityEditorRow::PowerWatts => {
            let choices = power_watt_choices(config.capability.power_watts);
            config.capability.power_watts =
                cycle_choice(config.capability.power_watts, &choices, direction);
        }
        SourceCapabilityEditorRow::Pd => {
            let next = cycle_choice(
                protocol_enabled(&config.capability.protocols, "pd"),
                &[false, true],
                direction,
            );
            set_protocol_flag(&mut config.capability.protocols, "pd", next)?;
        }
        SourceCapabilityEditorRow::Pps => {
            config.capability.pd.pps =
                cycle_choice(config.capability.pd.pps, &[false, true], direction);
        }
        SourceCapabilityEditorRow::Qc20 => {
            let next = cycle_choice(
                protocol_enabled(&config.capability.protocols, "qc20"),
                &[false, true],
                direction,
            );
            set_protocol_flag(&mut config.capability.protocols, "qc20", next)?;
        }
        SourceCapabilityEditorRow::Qc30 => {
            let next = cycle_choice(
                protocol_enabled(&config.capability.protocols, "qc30"),
                &[false, true],
                direction,
            );
            set_protocol_flag(&mut config.capability.protocols, "qc30", next)?;
        }
        SourceCapabilityEditorRow::Fcp => {
            let next = cycle_choice(
                protocol_enabled(&config.capability.protocols, "fcp"),
                &[false, true],
                direction,
            );
            set_protocol_flag(&mut config.capability.protocols, "fcp", next)?;
        }
        SourceCapabilityEditorRow::Afc => {
            let next = cycle_choice(
                protocol_enabled(&config.capability.protocols, "afc"),
                &[false, true],
                direction,
            );
            set_protocol_flag(&mut config.capability.protocols, "afc", next)?;
        }
        SourceCapabilityEditorRow::Scp => {
            let next = cycle_choice(
                protocol_enabled(&config.capability.protocols, "scp"),
                &[false, true],
                direction,
            );
            set_protocol_flag(&mut config.capability.protocols, "scp", next)?;
        }
        SourceCapabilityEditorRow::Pe20 => {
            let next = cycle_choice(
                protocol_enabled(&config.capability.protocols, "pe20"),
                &[false, true],
                direction,
            );
            set_protocol_flag(&mut config.capability.protocols, "pe20", next)?;
        }
        SourceCapabilityEditorRow::Bc12 => {
            let next = cycle_choice(
                protocol_enabled(&config.capability.protocols, "bc12"),
                &[false, true],
                direction,
            );
            set_protocol_flag(&mut config.capability.protocols, "bc12", next)?;
        }
        SourceCapabilityEditorRow::Sfcp => {
            let next = cycle_choice(
                protocol_enabled(&config.capability.protocols, "sfcp"),
                &[false, true],
                direction,
            );
            set_protocol_flag(&mut config.capability.protocols, "sfcp", next)?;
        }
        SourceCapabilityEditorRow::FixedPd => {
            state.fixed_pd_focus =
                cycle_index(state.fixed_pd_focus, FIXED_PD_OPTIONS.len(), direction);
        }
        SourceCapabilityEditorRow::Pps3Limit => {
            config.capability.current.pps3_limit_ma = cycle_choice(
                config.capability.current.pps3_limit_ma,
                &[3000, 5000],
                direction,
            );
        }
        SourceCapabilityEditorRow::PdPps5a => {
            config.capability.current.pd_pps_5a = cycle_choice(
                config.capability.current.pd_pps_5a,
                &[false, true],
                direction,
            );
        }
        SourceCapabilityEditorRow::TypeCBroadcast => {
            config.capability.current.type_c_broadcast_ma = cycle_choice(
                config.capability.current.type_c_broadcast_ma,
                &[500, 1500],
                direction,
            );
        }
        SourceCapabilityEditorRow::ScpLimit => {
            config.capability.current.scp_limit_ma = cycle_choice(
                config.capability.current.scp_limit_ma,
                &[2000, 4000, 5000],
                direction,
            );
        }
        SourceCapabilityEditorRow::FcpAfcSfcpLimit => {
            config.capability.current.fcp_afc_sfcp_limit_ma = cycle_choice(
                config.capability.current.fcp_afc_sfcp_limit_ma,
                &[2250, 3250],
                direction,
            );
        }
        SourceCapabilityEditorRow::Actions => {
            state.action_focus = cycle_index(state.action_focus, ACTION_OPTIONS.len(), direction);
        }
    }
    Ok(())
}

enum EditorSubmit {
    Continue,
    Save,
    Reload,
    Cancel,
}

fn select_choice<T: Copy + PartialEq>(
    prompt: &str,
    choices: &[T],
    current: T,
    mut label: impl FnMut(T) -> String,
) -> anyhow::Result<Option<T>> {
    let items = choices.iter().copied().map(&mut label).collect::<Vec<_>>();
    let default = choices
        .iter()
        .position(|value| *value == current)
        .unwrap_or(0);
    Ok(Select::new()
        .with_prompt(prompt)
        .items(&items)
        .default(default)
        .interact_opt()?
        .map(|index| choices[index]))
}

fn source_capability_row_label(config: &CliPowerConfig, row: SourceCapabilityEditorRow) -> String {
    match row {
        SourceCapabilityEditorRow::PowerWatts => {
            format!("Power cap: {} W", config.capability.power_watts)
        }
        SourceCapabilityEditorRow::Pd => format!(
            "PD: {}",
            on_off(protocol_enabled(&config.capability.protocols, "pd"))
        ),
        SourceCapabilityEditorRow::Pps => format!("PPS: {}", on_off(config.capability.pd.pps)),
        SourceCapabilityEditorRow::Qc20 => format!(
            "QC2.0: {}",
            on_off(protocol_enabled(&config.capability.protocols, "qc20"))
        ),
        SourceCapabilityEditorRow::Qc30 => format!(
            "QC3.0: {}",
            on_off(protocol_enabled(&config.capability.protocols, "qc30"))
        ),
        SourceCapabilityEditorRow::Fcp => format!(
            "FCP: {}",
            on_off(protocol_enabled(&config.capability.protocols, "fcp"))
        ),
        SourceCapabilityEditorRow::Afc => format!(
            "AFC: {}",
            on_off(protocol_enabled(&config.capability.protocols, "afc"))
        ),
        SourceCapabilityEditorRow::Scp => format!(
            "SCP: {}",
            on_off(protocol_enabled(&config.capability.protocols, "scp"))
        ),
        SourceCapabilityEditorRow::Pe20 => format!(
            "PE2.0: {}",
            on_off(protocol_enabled(&config.capability.protocols, "pe20"))
        ),
        SourceCapabilityEditorRow::Bc12 => format!(
            "BC1.2: {}",
            on_off(protocol_enabled(&config.capability.protocols, "bc12"))
        ),
        SourceCapabilityEditorRow::Sfcp => format!(
            "SFCP: {}",
            on_off(protocol_enabled(&config.capability.protocols, "sfcp"))
        ),
        SourceCapabilityEditorRow::FixedPd => {
            let enabled = FIXED_PD_OPTIONS
                .iter()
                .filter(|value| config.capability.pd.fixed_voltages_mv.contains(value))
                .map(|value| format!("{} V", value / 1000))
                .collect::<Vec<_>>();
            let summary = if enabled.is_empty() {
                "none".to_string()
            } else {
                enabled.join(", ")
            };
            format!("Fixed PD voltages: {summary}")
        }
        SourceCapabilityEditorRow::Pps3Limit => {
            format!("PPS3 limit: {} mA", config.capability.current.pps3_limit_ma)
        }
        SourceCapabilityEditorRow::PdPps5a => {
            format!(
                "PD/PPS 5 A: {}",
                on_off(config.capability.current.pd_pps_5a)
            )
        }
        SourceCapabilityEditorRow::TypeCBroadcast => format!(
            "Type-C current: {} mA",
            config.capability.current.type_c_broadcast_ma
        ),
        SourceCapabilityEditorRow::ScpLimit => {
            format!("SCP current: {} mA", config.capability.current.scp_limit_ma)
        }
        SourceCapabilityEditorRow::FcpAfcSfcpLimit => format!(
            "FCP/AFC/SFCP current: {} mA",
            config.capability.current.fcp_afc_sfcp_limit_ma
        ),
        SourceCapabilityEditorRow::Actions => "Actions: save, reload, cancel".to_string(),
    }
}

fn edit_source_capability_row(
    config: &mut CliPowerConfig,
    row: SourceCapabilityEditorRow,
) -> anyhow::Result<Option<EditorSubmit>> {
    match row {
        SourceCapabilityEditorRow::PowerWatts => {
            if let Some(value) = select_choice(
                "Power cap",
                &power_watt_choices(config.capability.power_watts),
                config.capability.power_watts,
                |value| format!("{value} W"),
            )? {
                config.capability.power_watts = value;
            }
            Ok(None)
        }
        SourceCapabilityEditorRow::Pd => {
            if let Some(value) = select_choice(
                "PD",
                &[false, true],
                protocol_enabled(&config.capability.protocols, "pd"),
                |value| on_off(value).to_string(),
            )? {
                set_protocol_flag(&mut config.capability.protocols, "pd", value)?;
            }
            Ok(None)
        }
        SourceCapabilityEditorRow::Pps => {
            if let Some(value) =
                select_choice("PPS", &[false, true], config.capability.pd.pps, |value| {
                    on_off(value).to_string()
                })?
            {
                config.capability.pd.pps = value;
            }
            Ok(None)
        }
        SourceCapabilityEditorRow::Qc20 => {
            if let Some(value) = select_choice(
                "QC2.0",
                &[false, true],
                protocol_enabled(&config.capability.protocols, "qc20"),
                |value| on_off(value).to_string(),
            )? {
                set_protocol_flag(&mut config.capability.protocols, "qc20", value)?;
            }
            Ok(None)
        }
        SourceCapabilityEditorRow::Qc30 => {
            if let Some(value) = select_choice(
                "QC3.0",
                &[false, true],
                protocol_enabled(&config.capability.protocols, "qc30"),
                |value| on_off(value).to_string(),
            )? {
                set_protocol_flag(&mut config.capability.protocols, "qc30", value)?;
            }
            Ok(None)
        }
        SourceCapabilityEditorRow::Fcp => {
            if let Some(value) = select_choice(
                "FCP",
                &[false, true],
                protocol_enabled(&config.capability.protocols, "fcp"),
                |value| on_off(value).to_string(),
            )? {
                set_protocol_flag(&mut config.capability.protocols, "fcp", value)?;
            }
            Ok(None)
        }
        SourceCapabilityEditorRow::Afc => {
            if let Some(value) = select_choice(
                "AFC",
                &[false, true],
                protocol_enabled(&config.capability.protocols, "afc"),
                |value| on_off(value).to_string(),
            )? {
                set_protocol_flag(&mut config.capability.protocols, "afc", value)?;
            }
            Ok(None)
        }
        SourceCapabilityEditorRow::Scp => {
            if let Some(value) = select_choice(
                "SCP",
                &[false, true],
                protocol_enabled(&config.capability.protocols, "scp"),
                |value| on_off(value).to_string(),
            )? {
                set_protocol_flag(&mut config.capability.protocols, "scp", value)?;
            }
            Ok(None)
        }
        SourceCapabilityEditorRow::Pe20 => {
            if let Some(value) = select_choice(
                "PE2.0",
                &[false, true],
                protocol_enabled(&config.capability.protocols, "pe20"),
                |value| on_off(value).to_string(),
            )? {
                set_protocol_flag(&mut config.capability.protocols, "pe20", value)?;
            }
            Ok(None)
        }
        SourceCapabilityEditorRow::Bc12 => {
            if let Some(value) = select_choice(
                "BC1.2",
                &[false, true],
                protocol_enabled(&config.capability.protocols, "bc12"),
                |value| on_off(value).to_string(),
            )? {
                set_protocol_flag(&mut config.capability.protocols, "bc12", value)?;
            }
            Ok(None)
        }
        SourceCapabilityEditorRow::Sfcp => {
            if let Some(value) = select_choice(
                "SFCP",
                &[false, true],
                protocol_enabled(&config.capability.protocols, "sfcp"),
                |value| on_off(value).to_string(),
            )? {
                set_protocol_flag(&mut config.capability.protocols, "sfcp", value)?;
            }
            Ok(None)
        }
        SourceCapabilityEditorRow::FixedPd => {
            let items = FIXED_PD_OPTIONS
                .iter()
                .map(|value| format!("{} V", value / 1000))
                .collect::<Vec<_>>();
            let defaults = FIXED_PD_OPTIONS
                .iter()
                .map(|value| config.capability.pd.fixed_voltages_mv.contains(value))
                .collect::<Vec<_>>();
            if let Some(selected) = MultiSelect::new()
                .with_prompt("Fixed PD voltages")
                .items(&items)
                .defaults(&defaults)
                .interact_opt()?
            {
                config.capability.pd.fixed_voltages_mv = selected
                    .into_iter()
                    .map(|index| FIXED_PD_OPTIONS[index])
                    .collect();
            }
            Ok(None)
        }
        SourceCapabilityEditorRow::Pps3Limit => {
            if let Some(value) = select_choice(
                "PPS3 limit",
                &[3000_u16, 5000],
                config.capability.current.pps3_limit_ma,
                |value| format!("{value} mA"),
            )? {
                config.capability.current.pps3_limit_ma = value;
            }
            Ok(None)
        }
        SourceCapabilityEditorRow::PdPps5a => {
            if let Some(value) = select_choice(
                "PD/PPS 5 A",
                &[false, true],
                config.capability.current.pd_pps_5a,
                |value| on_off(value).to_string(),
            )? {
                config.capability.current.pd_pps_5a = value;
            }
            Ok(None)
        }
        SourceCapabilityEditorRow::TypeCBroadcast => {
            if let Some(value) = select_choice(
                "Type-C current",
                &[500_u16, 1500],
                config.capability.current.type_c_broadcast_ma,
                |value| format!("{value} mA"),
            )? {
                config.capability.current.type_c_broadcast_ma = value;
            }
            Ok(None)
        }
        SourceCapabilityEditorRow::ScpLimit => {
            if let Some(value) = select_choice(
                "SCP current",
                &[2000_u16, 4000, 5000],
                config.capability.current.scp_limit_ma,
                |value| format!("{value} mA"),
            )? {
                config.capability.current.scp_limit_ma = value;
            }
            Ok(None)
        }
        SourceCapabilityEditorRow::FcpAfcSfcpLimit => {
            if let Some(value) = select_choice(
                "FCP/AFC/SFCP current",
                &[2250_u16, 3250],
                config.capability.current.fcp_afc_sfcp_limit_ma,
                |value| format!("{value} mA"),
            )? {
                config.capability.current.fcp_afc_sfcp_limit_ma = value;
            }
            Ok(None)
        }
        SourceCapabilityEditorRow::Actions => {
            let actions = ACTION_OPTIONS
                .iter()
                .map(|value| (*value).to_string())
                .collect::<Vec<_>>();
            let selected = Select::new()
                .with_prompt("Choose an action")
                .items(&actions)
                .default(0)
                .interact_opt()?;
            Ok(selected.map(|index| match index {
                0 => EditorSubmit::Save,
                1 => EditorSubmit::Reload,
                _ => EditorSubmit::Cancel,
            }))
        }
    }
}

fn submit_editor_row(
    config: &mut CliPowerConfig,
    state: &mut SourceCapabilityEditorState,
) -> anyhow::Result<EditorSubmit> {
    Ok(match SOURCE_CAPABILITY_EDITOR_ROWS[state.selected_row] {
        SourceCapabilityEditorRow::FixedPd => {
            toggle_fixed_pd_voltage(config, FIXED_PD_OPTIONS[state.fixed_pd_focus]);
            EditorSubmit::Continue
        }
        SourceCapabilityEditorRow::Actions => match state.action_focus {
            0 => EditorSubmit::Save,
            1 => EditorSubmit::Reload,
            _ => EditorSubmit::Cancel,
        },
        _ => {
            apply_row_direction(config, state, 1)?;
            EditorSubmit::Continue
        }
    })
}

fn run_source_capability_editor_tui(
    config: &mut CliPowerConfig,
    diagnostics: &str,
) -> anyhow::Result<EditorSubmit> {
    let mut selected_row = 0usize;
    loop {
        println!();
        println!("Source capability");
        println!("{}", truncate_lines(diagnostics, 6));
        let items = SOURCE_CAPABILITY_EDITOR_ROWS
            .iter()
            .map(|row| source_capability_row_label(config, *row))
            .collect::<Vec<_>>();
        let selected = Select::new()
            .with_prompt("Choose a field to edit")
            .items(&items)
            .default(selected_row.min(items.len().saturating_sub(1)))
            .interact_opt()?;
        let Some(selected) = selected else {
            return Ok(EditorSubmit::Cancel);
        };
        selected_row = selected;
        if let Some(submit) =
            edit_source_capability_row(config, SOURCE_CAPABILITY_EDITOR_ROWS[selected])?
        {
            if !matches!(submit, EditorSubmit::Continue) {
                return Ok(submit);
            }
        }
    }
}
