use eframe::egui::{self, Color32, Context, Key, Vec2};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Serialize, Deserialize, Clone)]
struct Task {
    text: String,
    priority: u8,
    color: [u8; 4], // RGBA color array
    selected: bool,

    #[serde(skip)]
    editing: bool,
}

struct MyApp {
    tasks: Vec<Task>,
    new_task_text: String,
    new_task_priority: u8,
    new_task_color: Color32,
    last_save: Instant,
    last_deleted_tasks: Vec<Task>,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            tasks: Vec::new(),
            new_task_text: String::new(),
            new_task_priority: 1,
            new_task_color: Color32::WHITE,
            last_save: Instant::now(),
            last_deleted_tasks: Vec::new(),
        }
    }
}

// Convert Color32 <-> [u8; 4]
fn color32_from_array(arr: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(arr[0], arr[1], arr[2], arr[3])
}

fn array_from_color32(color: Color32) -> [u8; 4] {
    [color.r(), color.g(), color.b(), color.a()]
}

impl MyApp {
    fn load_tasks() -> Vec<Task> {
        if let Some(path) = get_data_path() {
            if let Ok(data) = fs::read_to_string(path) {
                if let Ok(tasks) = serde_json::from_str(&data) {
                    return tasks;
                }
            }
        }
        Vec::new()
    }

    fn persist_tasks(&self) {
        if let Some(path) = get_data_path() {
            if let Ok(serialized) = serde_json::to_string_pretty(&self.tasks) {
                fs::write(path, serialized).ok();
            }
        }
    }

    fn add_task(&mut self) {
        if self.new_task_text.trim().is_empty() {
            return;
        }

        self.tasks.push(Task {
            text: self.new_task_text.trim().to_string(),
            priority: self.new_task_priority,
            color: array_from_color32(self.new_task_color),
            selected: false,
            editing: false,
        });

        // Sort tasks by priority descending (higher priority first)
        self.tasks.sort_by(|a, b| b.priority.cmp(&a.priority));

        self.new_task_text.clear();
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        if now.duration_since(self.last_save).as_secs() > 30 {
            self.persist_tasks();
            self.last_save = now;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let text = "📋 Tasks";
            let font_id = egui::FontId::proportional(32.0);

            // Draw shadow
            ui.painter().text(
                ui.min_rect().center_top() + egui::vec2(2.0, 2.0),
                egui::Align2::CENTER_TOP,
                text,
                font_id.clone(),
                Color32::from_rgba_unmultiplied(0, 0, 0, 150),
            );

            // Draw main colored text
            ui.painter().text(
                ui.min_rect().center_top(),
                egui::Align2::CENTER_TOP,
                text,
                font_id,
                Color32::from_rgb(0, 150, 255),
            );

            ui.add_space(50.0);

            ui.horizontal(|ui| {
                ui.label("Task:");
                ui.text_edit_singleline(&mut self.new_task_text);

                ui.label("Priority:");
                ui.add(
                    egui::DragValue::new(&mut self.new_task_priority)
                        .clamp_range(1..=10)
                        .speed(1),
                );

                let mut color_arr = array_from_color32(self.new_task_color);
                ui.color_edit_button_srgba_unmultiplied(&mut color_arr);
                self.new_task_color = color32_from_array(color_arr);

                if ui.button("➕ Add").clicked() {
                    self.add_task();
                }
            });

            ui.add_space(12.0);

            // Color presets
            ui.horizontal(|ui| {
                let presets = [
                    Color32::LIGHT_GREEN,
                    Color32::LIGHT_YELLOW,
                    Color32::LIGHT_RED,
                    Color32::LIGHT_BLUE,
                    Color32::WHITE,
                    Color32::DARK_RED,
                    Color32::DARK_GREEN,
                ];
                for &color in &presets {
                    if ui
                        .add(
                            egui::Button::new("   ")
                                .fill(color)
                                .frame(true)
                                .min_size(Vec2::new(24.0, 24.0)),
                        )
                        .clicked()
                    {
                        for task in self.tasks.iter_mut().filter(|t| t.selected) {
                            task.color = [color.r(), color.g(), color.b(), color.a()];
                        }
                        self.new_task_color = color;
                    }
                }
            });

            ui.add_space(16.0);

            // Keyboard navigation
            let selected_idx = self.tasks.iter().position(|t| t.selected);
            if ui.input(|i| i.key_pressed(Key::J)) {
                if let Some(i) = selected_idx {
                    if i + 1 < self.tasks.len() {
                        self.tasks[i].selected = false;
                        self.tasks[i + 1].selected = true;
                    }
                } else if !self.tasks.is_empty() {
                    self.tasks[0].selected = true;
                }
            }

            if ui.input(|i| i.key_pressed(Key::K)) {
                if let Some(i) = selected_idx {
                    if i > 0 {
                        self.tasks[i].selected = false;
                        self.tasks[i - 1].selected = true;
                    }
                } else if !self.tasks.is_empty() {
                    self.tasks[0].selected = true;
                }
            }

            if ui.input(|i| i.key_pressed(Key::D)) {
                self.last_deleted_tasks =
                    self.tasks.iter().filter(|t| t.selected).cloned().collect();

                self.tasks.retain(|t| !t.selected);
            }

            // Undo last delete on pressing U
            if ui.input(|i| i.key_pressed(Key::U)) {
                if !self.last_deleted_tasks.is_empty() {
                    self.tasks.append(&mut self.last_deleted_tasks);
                    self.tasks.sort_by(|a, b| b.priority.cmp(&a.priority));
                    self.last_deleted_tasks.clear();
                }
            }

            // Show tasks
            let mut drag_index: Option<usize> = None;
            let mut drop_index: Option<usize> = None;

            for (i, task) in self.tasks.iter_mut().enumerate() {
                egui::Frame::none()
                    .fill(color32_from_array(task.color))
                    .stroke(if task.selected {
                        egui::Stroke::new(3.0, Color32::YELLOW)
                    } else {
                        egui::Stroke::new(1.0, Color32::BLACK)
                    })
                    .rounding(egui::Rounding::same(8.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.add_space(6.0);

                            

                            egui::Frame::none()
                                .fill(Color32::BLACK)
                                .stroke(egui::Stroke::new(1.5, Color32::WHITE))
                                .rounding(egui::Rounding::same(6.0))
                                .show(ui, |ui| {
                                    let priority_size = Vec2::new(30.0, 30.0);
                                    ui.allocate_ui(priority_size, |ui| {
                                        ui.centered_and_justified(|ui| {
                                            ui.label(
                                                egui::RichText::new(task.priority.to_string())
                                                    .color(Color32::WHITE)
                                                    .size(22.0),
                                            );
                                        });
                                    });
                                });

                            ui.add_space(10.0);

                            let available_width = ui.available_width();

                            if task.editing {
                                // Show editable text box bound to task.text
                                let response = ui.add_sized(
                                    Vec2::new(available_width, 30.0),
                                    egui::TextEdit::singleline(&mut task.text)
                                        .font(egui::FontId::proportional(20.0))
                                        .desired_width(f32::INFINITY),
                                );

                                // Exit edit mode on Enter
                                if response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter))
                                {
                                    task.editing = false;
                                }
                            } else {
                                // Show task text normally, centered
                                let response = ui.allocate_response(
                                    Vec2::new(available_width, 40.0),
                                    egui::Sense::click_and_drag(),
                                );

                                ui.painter().text(
                                    response.rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    &task.text,
                                    egui::FontId::proportional(20.0),
                                    Color32::BLACK,
                                );

                                if response.double_clicked() {
                                    task.editing = true;
                                }

                                if response.clicked() {
                                    task.selected = !task.selected;
                                }

                                if response.drag_started() {
                                    drag_index = Some(i);
                                }

                                if response.hovered() && ui.input(|i| i.pointer.any_released()) {
                                    drop_index = Some(i);
                                }
                            }
                        });
                    });

                ui.add_space(4.0);
            }

            if let (Some(from), Some(to)) = (drag_index, drop_index) {
                if from != to {
                    let task = self.tasks.remove(from);
                    self.tasks.insert(to, task);
                }
            }

            ui.add_space(12.0);

            // Trash button
            let any_selected = self.tasks.iter().any(|t| t.selected);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::BOTTOM), |ui| {
                if ui
                    .add_enabled(any_selected, egui::Button::new("🗑 Delete selected"))
                    .clicked()
                {
                    self.tasks.retain(|t| !t.selected);
                }
            });
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.persist_tasks();
    }
}

fn get_data_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|mut path| {
        path.push("rust_tasks.json");
        path
    })
}

fn main() -> eframe::Result<()> {
    let mut options = eframe::NativeOptions::default();

    // Set initial window size here
    options.initial_window_size = Some(Vec2::new(550.0, 450.0));

    eframe::run_native(
        "Rust Task Manager",
        options,
        Box::new(|_cc| {
            Box::new(MyApp {
                tasks: MyApp::load_tasks(),
                last_save: Instant::now(),
                ..Default::default()
            })
        }),
    )
}
