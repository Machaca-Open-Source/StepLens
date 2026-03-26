use egui::*;
use crate::models::Step;
use image::{EncodableLayout, load_from_memory, imageops, DynamicImage};

pub struct StepListView;

impl StepListView {
    pub fn show(
        ui: &mut Ui,
        steps: &[Step],
        selected_index: &mut Option<usize>,
        _screenshot_dir: &std::path::Path,
    ) {
        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                for (index, step) in steps.iter().enumerate() {
                    let is_selected = selected_index == &Some(index);

                    ui.horizontal(|ui| {
                        // Thumbnail
                        if step.screenshot_path.exists() {
                            if let Ok(img_data) = std::fs::read(&step.screenshot_path) {
                                if let Ok(img) = load_from_memory(&img_data) {
                                    let thumbnail = imageops::resize(
                                        &img,
                                        100,
                                        75,
                                        imageops::FilterType::Lanczos3,
                                    );
                                    
                                    // Convert to RGB format for egui (from DynamicImage)
                                    let rgb_thumbnail = DynamicImage::ImageRgba8(thumbnail).to_rgb8();
                                    
                                    let size = Vec2::new(100.0, 75.0);
                                    let color_image = ColorImage::from_rgb(
                                        [rgb_thumbnail.width() as usize, rgb_thumbnail.height() as usize],
                                        rgb_thumbnail.as_bytes(),
                                    );
                                    let texture = ui.ctx().load_texture(
                                        format!("step_{}", index),
                                        color_image,
                                        Default::default(),
                                    );
                                    
                                    ui.image((texture.id(), size));
                                }
                            }
                        }

                        // Step info
                        ui.vertical(|ui| {
                            if ui
                                .selectable_label(is_selected, &step.title)
                                .clicked()
                            {
                                *selected_index = Some(index);
                            }

                            ui.label(
                                RichText::new(
                                    chrono::DateTime::from_timestamp(step.timestamp, 0)
                                        .unwrap_or_default()
                                        .format("%H:%M:%S")
                                        .to_string(),
                                )
                                .small()
                                .weak(),
                            );
                        });
                    });

                    ui.separator();
                }
            });
    }
}

pub struct StepEditor;

impl StepEditor {
    pub fn show(ui: &mut Ui, step: Option<&mut Step>, show_ai_button: bool) -> bool {
        let mut ai_clicked = false;
        match step {
            Some(step) => {
                // Screenshot preview
                if step.screenshot_path.exists() {
                    if let Ok(img_data) = std::fs::read(&step.screenshot_path) {
                        if let Ok(img) = load_from_memory(&img_data) {
                            let max_width = ui.available_width();
                            let scale = if img.width() as f32 > max_width {
                                max_width / img.width() as f32
                            } else {
                                1.0
                            };

                            let display_width = (img.width() as f32 * scale).min(max_width) as u32;
                            let display_height = (img.height() as f32 * scale) as u32;

                            let resized = imageops::resize(
                                &img,
                                display_width,
                                display_height,
                                imageops::FilterType::Lanczos3,
                            );

                            // Convert to RGB format for egui (from DynamicImage)
                            let rgb_resized = DynamicImage::ImageRgba8(resized).to_rgb8();

                            let color_image = ColorImage::from_rgb(
                                [rgb_resized.width() as usize, rgb_resized.height() as usize],
                                rgb_resized.as_bytes(),
                            );
                            let texture = ui.ctx().load_texture(
                                format!("preview_{}", step.id),
                                color_image,
                                Default::default(),
                            );

                            ui.image((texture.id(), Vec2::new(display_width as f32, display_height as f32)));
                            ui.add_space(10.0);
                        }
                    }
                }

                // Title editor
                ui.label("Title:");
                ui.text_edit_singleline(&mut step.title);
                ui.add_space(10.0);

                // Description editor
                ui.label("Description:");
                ui.add(TextEdit::multiline(&mut step.description).desired_rows(10));
                ui.add_space(10.0);

                // AI Generate button
                if show_ai_button {
                    if ui.button("Generate Instructions with AI").clicked() {
                        ai_clicked = true;
                    }
                }
            }
            None => {
                ui.centered_and_justified(|ui| {
                    ui.label("Select a step to edit");
                });
            }
        }
        ai_clicked
    }
}

pub struct TopBar;

impl TopBar {
    pub fn show(
        ui: &mut Ui,
        is_recording: bool,
        start_recording_cb: impl FnOnce() + 'static,
        stop_recording_cb: impl FnOnce() + 'static,
        export_docx_cb: impl FnOnce() + 'static,
        export_pdf_cb: impl FnOnce() + 'static,
    ) {
        ui.horizontal(|ui| {
            if is_recording {
                if ui.button("Stop Recording").clicked() {
                    stop_recording_cb();
                }
                ui.label(RichText::new("● Recording...").color(Color32::RED));
            } else {
                if ui.button("Start Recording").clicked() {
                    start_recording_cb();
                }
            }

            ui.separator();

            ui.menu_button("Export", |ui| {
                if ui.button("Export to DOCX").clicked() {
                    export_docx_cb();
                }
                if ui.button("Export to PDF").clicked() {
                    export_pdf_cb();
                }
            });
        });
    }
}

