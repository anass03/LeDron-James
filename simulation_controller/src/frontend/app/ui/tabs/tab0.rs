use std::env::SplitPaths;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Widget, Paragraph, Wrap};
use tui_logger::{TuiLoggerSmartWidget, TuiLoggerWidget};
use wg_2024::network::NodeId;
use crate::frontend::app::App;
use crate::frontend::app::ui::tabs::{MAIN_BLOCK_BORDERS, BG_COLOR};
use tui_logger::{ExtLogRecord, LogFormatter};
use ratatui::text::{Line, Span, Text};
use log::Level;
use ratatui::widgets::canvas::{Context, Circle, Line as CanvasLine, Canvas};
use textwrap;
use crate::frontend::app::ui::tabs::tab1::data_rendering::render_most_active_nodes_chart;


/// Renders Overview tab-page
pub(super) fn render_tab0(app: &mut App, area: Rect, buf : &mut Buffer){
    let [left_area,right_area] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(70), // CLI / Log messages
        ])
        .areas(area);
    let [help_area,log_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .areas(right_area);
    let [graph1, graph2, graph3] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(33), Constraint::Percentage(34), Constraint::Percentage(33)])
        .areas(left_area);

    // Render the dynamic help system
    render_dynamic_help(app, help_area, buf);

    use crate::frontend::app::ui::tabs; // Statistic Block
    use super::super::utils::chart_rendering;

    let mut chart1_block = Block::default()
        .title(" Dropped Messages ")
        .title_alignment(Alignment::Left)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(
            if app.cache.cursor == 0 {
                app.animation_color()
            }else{
                MAIN_BLOCK_BORDERS
            }
        ));
    chart_rendering::render_sparkline_1(&mut app.cache.tab0_data.charts_data,chart1_block,graph1,buf);

    let mut chart2_block = Block::default()
        .title(" Sent Messages ")
        .title_alignment(Alignment::Left)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(
            if app.cache.cursor == 1 {
                app.animation_color()
            }else{
                MAIN_BLOCK_BORDERS
            }
        ));
    chart_rendering::render_sparkline_2(&mut app.cache.tab0_data.charts_data, chart2_block, graph2, buf);

    let mut chart3_block = Block::default()
        .title(" Most active nodes ")
        .title_alignment(Alignment::Left)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(
            if app.cache.cursor == 2 {
                app.animation_color()
            }else{
                MAIN_BLOCK_BORDERS
            }
        ));
    let inner_chart3 = chart3_block.inner(graph3);
    chart3_block.render(graph3, buf);
    // Render the most active nodes bar chart
    render_most_active_nodes_chart(app, inner_chart3, buf);


    // Shitty performance, gotta improve it with a Tick Rate/*
    /*match app.cache.tab0_data.selected_chart{
        0 => { chart_rendering::render_sparkline_1(&app.cache.tab0_data.charts_data,chart_block,graph_area,buf) },
        1 => {  },
        2 => {  },
        _ => panic!("OVERFLOW CHART SELECTION!")
    }*/

    let log_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(
            if app.cache.cursor == 3 {
                app.animation_color()
            }else{
                MAIN_BLOCK_BORDERS
            }
        ))
        .border_type(BorderType::Rounded);
    let inner_log_block = log_block.inner(log_area);
    log_block.render(log_area, buf);

    TuiLoggerWidget::default()
        .style_error(Style::default().fg(Color::Red))
        .style_warn(Style::default().fg(Color::Yellow))
        .style_info(Style::default().fg(Color::White))
        .style_debug(Style::default().fg(Color::Green))
        .style_trace(Style::default().fg(Color::Magenta))
        .opt_formatter(Some(Box::new(MyLogFormatter{})))
        .output_separator(' ')
        .output_timestamp(Some("%H:%M:%S".to_string()))
        .output_target(false)
        .output_file(false)
        .output_line(false)
        .state(&app.cache.main_log_state.1)
        .render(inner_log_block, buf);
}

struct MyLogFormatter {}
impl LogFormatter for MyLogFormatter {
    fn min_width(&self) -> u16 {
        4
    }
    fn format(&self, width: usize, evt: &ExtLogRecord) -> Vec<Line> {
        let mut lines = vec![];
        let identifier_str = match parse_node_string(evt.target()){
            None => {
                format!("Unknown Target")
            }
            Some((id,node)) => {
                format!("{:?} ID: {:?}", node,id)
            }
        };

        // Create the base log entry format
        let timestamp = evt.timestamp.format("%H:%M:%S").to_string();
        let level_str = format!("{:?}", evt.level);
        let level_short = match evt.level {
            log::Level::Error => "ERR",
            log::Level::Debug => "DBG", 
            log::Level::Warn => "WARN",
            log::Level::Info => "INFO",
            log::Level::Trace => "GEN",
        };

        // Calculate available width for the message (accounting for timestamp, level, and separators)
        let header_width = timestamp.len() + 3 + level_short.len() + 3; // "HH:MM:SS | [LEVEL] | "
        let available_width = width.saturating_sub(header_width);
        
        // Wrap the message to fit the available width
        let wrapped_message = textwrap::wrap(evt.msg(), available_width);
        
        // Get the appropriate color for the log level
        let color = match evt.level {
            log::Level::Error => Color::Rgb(255, 168, 168),
            log::Level::Debug => Color::Rgb(201, 255, 246),
            log::Level::Warn => Color::Rgb(253, 215, 170),
            log::Level::Info => Color::Rgb(182, 255, 206),
            log::Level::Trace => Color::Rgb(237, 254, 173),
        };

        // Create lines for each wrapped message part
        for (i, message_line) in wrapped_message.iter().enumerate() {
            let formatted_line = if i == 0 {
                // First line includes the full header
                format!("{} | [{}] | {}", timestamp, level_short, message_line)
            } else {
                // Subsequent lines just continue the message with proper indentation
                let indent_width = timestamp.len() + 3 + level_short.len() + 3; // Same as header_width
                format!("{}{}", " ".repeat(indent_width), message_line)
            };
            
            lines.push(Line::from(vec![
                Span::styled(formatted_line, Style::default().fg(color))
            ]));
        }
        
        lines
    }
}
fn parse_node_string(input: &str) -> Option<(NodeId, &str)> {
    let parts: Vec<&str> = input.split('_').collect();

    if parts.len() != 2 {
        return None;
    }

    let node_id = parts[0].parse::<u8>().ok()?;
    let role = parts[1];

    match role {
        "Server" | "Client" => Some((node_id, role)),
        _ => None,
    }
}

/// Renders dynamic help content that changes periodically
fn render_dynamic_help(app: &App, area: Rect, buf: &mut Buffer) {
    // Change help content every 100 frames (about 5 seconds at 20fps)
    let help_index = (app.cache.counter / 100) % 15;
    
    let help_content = get_help_content(help_index as u64);
    
    let help_block = Block::default()
        .title(" 💡 Did You Know? ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(
            if app.cache.cursor == 4 {
                app.animation_color()
            } else {
                MAIN_BLOCK_BORDERS
            }
        ));
    
    let inner_help_area = help_block.inner(area);
    help_block.render(area, buf);
    
    // Create animated title with sparkle effect
    let sparkle_frames = ["✨", "⭐", "💫", "🌟", "✨"];
    let sparkle = sparkle_frames[((app.cache.counter / 10 )as usize) % sparkle_frames.len()];
    
    let title_text = Text::from(vec![
        Line::from(vec![
            Span::styled(sparkle, Style::default().fg(Color::Yellow)),
            Span::styled(" Random Network Wisdom ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(sparkle, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(""),
    ]);
    
    let title_para = Paragraph::new(title_text)
        .alignment(Alignment::Center);
    
    let [title_area, content_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .areas(inner_help_area);
    
    title_para.render(title_area, buf);
    
    // Create particle effects canvas
    let particle_canvas = create_particle_canvas(app);
    particle_canvas.render(content_area, buf);
    
    // Render the main help content centered over the particles
    let content_para = Paragraph::new(help_content)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    
    content_para.render(content_area, buf);
}

/// Creates animated particle effects for the help area
fn create_particle_canvas(app: &App) -> Canvas<'static, fn(&mut Context)> {
    // Use a static function that gets the counter from a global state
    Canvas::default()
        .background_color(BG_COLOR)
        .x_bounds([0.0, 100.0])
        .y_bounds([0.0, 100.0])
        .paint(particle_painter)
}

/// Static function for painting particles (no captured variables)
fn particle_painter(ctx: &mut Context) {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    // Get time from system clock instead of captured variable
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let time = millis as f64 * 0.00015;
    
    // Move the center point higher up (from 50.0 to 35.0)
    let center_x = 50.0;
    let center_y = 35.0; // Moved up from 50.0
    
    // Floating light particles with rainbow effect
    for i in 0..12 {
        let angle = time * 0.3 + (i as f64 * 0.523); // 30 degrees apart
        let radius = 20.0 + (time * 0.2 + i as f64 * 0.5).sin() * 15.0;
        let x = center_x + angle.cos() * radius;
        let y = center_y + angle.sin() * radius;
        let size = 0.8 + (time + i as f64 * 0.7).sin() * 0.4;
        
        // Rainbow color cycling
        let hue = (time * 25.0 + i as f64 * 30.0) % 360.0;
        let (r, g, b) = hsv_to_rgb(hue, 0.8, 1.0);
        
        ctx.draw(&Circle {
            x, y, radius: size,
            color: Color::Rgb(r, g, b),
            ..Default::default()
        });
    }
    
    // Orbital rings with pulsing effect
    for ring in 0..3 {
        let ring_radius = 15.0 + ring as f64 * 8.0 + (time * 0.5).sin() * 3.0;
        let ring_alpha = (50 + (time * 30.0 + ring as f64 * 20.0).sin() as u8).min(150);
        ctx.draw(&Circle {
            x: center_x, y: center_y, radius: ring_radius,
            color: Color::Rgb(ring_alpha, ring_alpha, 255),
            ..Default::default()
        });
    }
    
    // Sparkle effects around the edges
    for sparkle in 0..8 {
        let sparkle_angle = time * 0.4 + sparkle as f64 * 0.785; // 45 degrees apart
        let sparkle_radius = 35.0 + (time * 0.3).sin() * 5.0;
        let sparkle_x = center_x + sparkle_angle.cos() * sparkle_radius;
        let sparkle_y = center_y + sparkle_angle.sin() * sparkle_radius;
        let sparkle_size = 0.3 + (time + sparkle as f64).sin() * 0.2;
        
        // White sparkles with glow
        ctx.draw(&Circle {
            x: sparkle_x, y: sparkle_y, radius: sparkle_size,
            color: Color::White,
            ..Default::default()
        });
    }
    
    // DNA-like helix effect in background
    for i in 0..15 {
        let angle = time * 0.2 + (i as f64 * 0.419); // 24 degrees apart
        let radius = 8.0 + (time * 0.4).sin() * 3.0;
        let x = center_x + angle.cos() * radius;
        let y = center_y + angle.sin() * radius + (i as f64 * 0.3);
        let size = 0.2 + (time + i as f64 * 0.1).sin() * 0.1;
        
        let helix_color = if i % 2 == 0 {
            Color::Rgb(100, 200, 255)
        } else {
            Color::Rgb(255, 100, 200)
        };
        
        ctx.draw(&Circle {
            x, y, radius: size,
            color: helix_color,
            ..Default::default()
        });
    }
    
    // Magnetic field lines
    for line in 0..6 {
        let line_angle = time * 0.1 + line as f64 * 1.047; // 60 degrees apart
        let start_x = center_x + line_angle.cos() * 5.0;
        let start_y = center_y + line_angle.sin() * 5.0;
        let end_x = center_x + line_angle.cos() * 45.0;
        let end_y = center_y + line_angle.sin() * 45.0;
        
        let field_intensity = (80 + (time * 20.0 + line as f64 * 15.0).sin() as u8).min(180);
        ctx.draw(&CanvasLine {
            x1: start_x, y1: start_y, x2: end_x, y2: end_y,
            color: Color::Rgb(field_intensity, field_intensity, 255),
            ..Default::default()
        });
    }
    
    // Quantum particles (smaller, faster particles)
    for quantum in 0..20 {
        let quantum_angle = time * 1.5 + quantum as f64 * 0.314; // 18 degrees apart
        let quantum_radius = 5.0 + (time * 0.8).sin() * 8.0;
        let quantum_x = center_x + quantum_angle.cos() * quantum_radius;
        let quantum_y = center_y + quantum_angle.sin() * quantum_radius;
        let quantum_size = 0.1 + (time * 1.2 + quantum as f64).sin() * 0.1;
        
        let quantum_hue = (time * 40.0 + quantum as f64 * 18.0) % 360.0;
        let (r, g, b) = hsv_to_rgb(quantum_hue, 0.9, 1.0);
        
        ctx.draw(&Circle {
            x: quantum_x, y: quantum_y, radius: quantum_size,
            color: Color::Rgb(r, g, b),
            ..Default::default()
        });
    }
    
    // Pulsing center glow
    let center_glow_radius = 12.0 + (time * 0.6).sin() * 4.0;
    let glow_intensity = (100 + (time * 40.0).sin() as u8).min(200);
    ctx.draw(&Circle {
        x: center_x, y: center_y, radius: center_glow_radius,
        color: Color::Rgb(glow_intensity, glow_intensity, 255),
        ..Default::default()
    });
}

/// Returns different help content based on the index
fn get_help_content(index: u64) -> Text<'static> {
    match index {
        0 => Text::from(vec![
            Line::from("🌐 Network Topologies").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Line::from(""),
            Line::from("Star Network: All nodes connect to a central hub.").style(Style::default().fg(Color::White)),
            Line::from("Tree Network: Hierarchical structure like a family tree.").style(Style::default().fg(Color::White)),
            Line::from("Butterfly Network: Multi-stage interconnection pattern.").style(Style::default().fg(Color::White)),
            Line::from(""),
            Line::from("💡 Each topology has different latency and bandwidth characteristics!").style(Style::default().fg(Color::Cyan)),
        ]),
        
        1 => Text::from(vec![
            Line::from("📡 Chat System").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Line::from(""),
            Line::from("• Messages are routed through chat servers").style(Style::default().fg(Color::White)),
            Line::from("• Media files can be attached and shared").style(Style::default().fg(Color::White)),
            Line::from("• Real-time message delivery with timestamps").style(Style::default().fg(Color::White)),
            Line::from(""),
            Line::from("💡 Press 'r' to discover new chat servers!").style(Style::default().fg(Color::Cyan)),
        ]),
        
        2 => Text::from(vec![
            Line::from("🖥️ Content Distribution").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Line::from(""),
            Line::from("• Files and media are stored on content servers").style(Style::default().fg(Color::White)),
            Line::from("• Progressive download with progress indicators").style(Style::default().fg(Color::White)),
            Line::from("• Support for text files and image rendering").style(Style::default().fg(Color::White)),
            Line::from(""),
            Line::from("💡 Large files show loading progress in real-time!").style(Style::default().fg(Color::Cyan)),
        ]),
        
        3 => Text::from(vec![
            Line::from("🤖 Drone Network").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Line::from(""),
            Line::from("• Drones act as network relays and routers").style(Style::default().fg(Color::White)),
            Line::from("• They can add/remove sender connections").style(Style::default().fg(Color::White)),
            Line::from("• Dynamic topology reconfiguration").style(Style::default().fg(Color::White)),
            Line::from(""),
            Line::from("💡 Drones help optimize network paths!").style(Style::default().fg(Color::Cyan)),
        ]),
        
        4 => Text::from(vec![
            Line::from("📊 Network Metrics").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Line::from(""),
            Line::from("• Dropped messages: Network reliability indicator").style(Style::default().fg(Color::White)),
            Line::from("• Sent messages: Network activity level").style(Style::default().fg(Color::White)),
            Line::from("• Active nodes: Shows which nodes are busy").style(Style::default().fg(Color::White)),
            Line::from(""),
            Line::from("💡 Monitor these to optimize your network!").style(Style::default().fg(Color::Cyan)),
        ]),
        
        5 => Text::from(vec![
            Line::from("🔧 Simulation Controls").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Line::from(""),
            Line::from("• Arrow keys: Navigate between tabs and options").style(Style::default().fg(Color::White)),
            Line::from("• Enter: Select items and confirm actions").style(Style::default().fg(Color::White)),
            Line::from("• 'r' key: Refresh network discovery").style(Style::default().fg(Color::White)),
            Line::from(""),
            Line::from("💡 Use Tab to switch between Menu and Option areas!").style(Style::default().fg(Color::Cyan)),
        ]),
        
        6 => Text::from(vec![
            Line::from("🌍 Network Discovery").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Line::from(""),
            Line::from("• Nodes automatically discover each other").style(Style::default().fg(Color::White)),
            Line::from("• Dynamic server and client detection").style(Style::default().fg(Color::White)),
            Line::from("• Real-time topology updates").style(Style::default().fg(Color::White)),
            Line::from(""),
            Line::from("💡 The network adapts as nodes join and leave!").style(Style::default().fg(Color::Cyan)),
        ]),
        
        7 => Text::from(vec![
            Line::from("📁 File Management").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Line::from(""),
            Line::from("• Upload files to share with the network").style(Style::default().fg(Color::White)),
            Line::from("• Download and preview content").style(Style::default().fg(Color::White)),
            Line::from("• Support for various file formats").style(Style::default().fg(Color::White)),
            Line::from(""),
            Line::from("💡 Text files render directly in the terminal!").style(Style::default().fg(Color::Cyan)),
        ]),
        
        8 => Text::from(vec![
            Line::from("🎨 UI Features").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Line::from(""),
            Line::from("• Animated borders and visual feedback").style(Style::default().fg(Color::White)),
            Line::from("• Color-coded status indicators").style(Style::default().fg(Color::White)),
            Line::from("• Responsive layout adaptation").style(Style::default().fg(Color::White)),
            Line::from(""),
            Line::from("💡 The interface updates in real-time!").style(Style::default().fg(Color::Cyan)),
        ]),
        
        9 => Text::from(vec![
            Line::from("🔐 Network Security").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Line::from(""),
            Line::from("• Secure message routing").style(Style::default().fg(Color::White)),
            Line::from("• Authenticated node connections").style(Style::default().fg(Color::White)),
            Line::from("• Protected file transfers").style(Style::default().fg(Color::White)),
            Line::from(""),
            Line::from("💡 All communications are encrypted!").style(Style::default().fg(Color::Cyan)),
        ]),
        
        10 => Text::from(vec![
            Line::from("⚡ Performance Tips").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Line::from(""),
            Line::from("• Use star topology for simple networks").style(Style::default().fg(Color::White)),
            Line::from("• Tree topology for hierarchical structures").style(Style::default().fg(Color::White)),
            Line::from("• Butterfly for high-bandwidth applications").style(Style::default().fg(Color::White)),
            Line::from(""),
            Line::from("💡 Choose topology based on your needs!").style(Style::default().fg(Color::Cyan)),
        ]),
        
        11 => Text::from(vec![
            Line::from("🔄 Real-time Updates").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Line::from(""),
            Line::from("• Live message delivery status").style(Style::default().fg(Color::White)),
            Line::from("• Dynamic network topology changes").style(Style::default().fg(Color::White)),
            Line::from("• Instant file transfer progress").style(Style::default().fg(Color::White)),
            Line::from(""),
            Line::from("💡 Everything updates without refreshing!").style(Style::default().fg(Color::Cyan)),
        ]),
        
        12 => Text::from(vec![
            Line::from("🎯 Advanced Features").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Line::from(""),
            Line::from("• Multi-tabbed interface for different functions").style(Style::default().fg(Color::White)),
            Line::from("• Popup windows for detailed views").style(Style::default().fg(Color::White)),
            Line::from("• Scrollable content areas").style(Style::default().fg(Color::White)),
            Line::from(""),
            Line::from("💡 Explore all tabs to discover features!").style(Style::default().fg(Color::Cyan)),
        ]),
        
        13 => Text::from(vec![
            Line::from("🔍 Debugging Tools").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Line::from(""),
            Line::from("• Comprehensive logging system").style(Style::default().fg(Color::White)),
            Line::from("• Network state inspection").style(Style::default().fg(Color::White)),
            Line::from("• Performance monitoring").style(Style::default().fg(Color::White)),
            Line::from(""),
            Line::from("💡 Check the log area for detailed information!").style(Style::default().fg(Color::Cyan)),
        ]),
        
        14 => Text::from(vec![
            Line::from("🚀 Future Features").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Line::from(""),
            Line::from("• Voice and video chat support").style(Style::default().fg(Color::White)),
            Line::from("• Advanced routing algorithms").style(Style::default().fg(Color::White)),
            Line::from("• Machine learning optimization").style(Style::default().fg(Color::White)),
            Line::from(""),
            Line::from("💡 This is just the beginning!").style(Style::default().fg(Color::Cyan)),
        ]),
        
        _ => Text::from(vec![
            Line::from("❓ Random Fact").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Line::from(""),
            Line::from("The first computer network was ARPANET,").style(Style::default().fg(Color::White)),
            Line::from("created in 1969 by the US Department of Defense.").style(Style::default().fg(Color::White)),
            Line::from("It was the precursor to the modern internet!").style(Style::default().fg(Color::White)),
            Line::from(""),
            Line::from("💡 History shapes the future of networking!").style(Style::default().fg(Color::Cyan)),
        ]),
    }
}

// Helper function to convert HSV to RGB for particle effects
fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (((r + m) * 255.0) as u8, ((g + m) * 255.0) as u8, ((b + m) * 255.0) as u8)
}




