use egui::{RichText, Color32};
use std::sync::mpsc;
use std::path::PathBuf;
use anyhow::Result;

use crate::models::{Recording, Step, EventType as AppEventType};
use crate::storage::Storage;
use crate::capture::ScreenshotCapturer;
use crate::hooks::{InputHook, InputEvent};
use crate::llm::LLMClient;
use crate::export::Exporter;
use crate::ui::{StepListView, StepEditor};

pub struct CapturerApp {
    storage: Storage,
    recording: Option<Recording>,
    is_recording: bool,
    input_hook: Option<InputHook>,
    event_receiver: Option<mpsc::Receiver<InputEvent>>,
    selected_step_index: Option<usize>,
    llm_client: LLMClient,
    screenshots_dir: PathBuf,
    generating_ai: bool,
    ai_result_sender: Option<mpsc::Sender<(usize, String)>>,
    ai_result_receiver: Option<mpsc::Receiver<(usize, String)>>,
    // Async screenshot capture
    screenshot_sender: Option<mpsc::Sender<(PathBuf, String)>>,  // (path, step_id)
    screenshot_receiver: Option<mpsc::Receiver<(PathBuf, String)>>,
    // UI state
    overlay_pos: egui::Pos2,
    capture_region_size: u32,
    pending_steps: Vec<(InputEvent, String)>, // Events waiting for screenshots
    overlay_collapsed: bool,
}

impl CapturerApp {
    pub fn new() -> Result<Self> {
        let storage = Storage::new()?;
        let screenshots_dir = storage.get_screenshots_dir();
        std::fs::create_dir_all(&screenshots_dir)?;

        let llm_client = LLMClient::new("llama3.2".to_string());

        let (ai_sender, ai_receiver) = mpsc::channel();
        let (screenshot_sender, screenshot_receiver) = mpsc::channel();

        Ok(Self {
            storage,
            recording: None,
            is_recording: false,
            input_hook: None,
            event_receiver: None,
            selected_step_index: None,
            llm_client,
            screenshots_dir,
            generating_ai: false,
            ai_result_sender: Some(ai_sender),
            ai_result_receiver: Some(ai_receiver),
            screenshot_sender: Some(screenshot_sender),
            screenshot_receiver: Some(screenshot_receiver),
            overlay_pos: egui::Pos2::new(50.0, 50.0),
            capture_region_size: 400,
            pending_steps: Vec::new(),
            overlay_collapsed: false,
        })
    }

    fn start_recording(&mut self) {
        if self.is_recording {
            return;
        }

        let name = format!(
            "Recording {}",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
        );
        self.recording = Some(Recording::new(name));
        self.is_recording = true;
        self.selected_step_index = None;

        eprintln!("DEBUG: Starting recording, setting up hooks...");
        let (hook, receiver) = InputHook::new();
        self.input_hook = Some(hook);
        self.event_receiver = Some(receiver);
        eprintln!("DEBUG: Recording started, hooks ready");
    }

    fn stop_recording(&mut self) {
        self.is_recording = false;
        self.input_hook = None;
        self.event_receiver = None;

        if let Some(ref recording) = self.recording {
            let _ = self.storage.save_recording(recording);
        }
    }

    fn process_input_events(&mut self) {
        // Collect events first to avoid borrow checker issues
        let mut events = Vec::new();
        if let Some(ref receiver) = self.event_receiver {
            while let Ok(event) = receiver.try_recv() {
                events.push(event);
            }
        }
        
        for event in events {
            self.handle_input_event(event);
        }

        // Process AI generation results
        if let Some(ref receiver) = self.ai_result_receiver {
            while let Ok((index, instruction)) = receiver.try_recv() {
                if let Some(ref mut recording) = self.recording {
                    if index < recording.steps.len() {
                        recording.steps[index].description = instruction;
                        if let Err(e) = self.storage.save_recording(recording) {
                            eprintln!("Failed to save recording: {}", e);
                        }
                    }
                }
                self.generating_ai = false;
            }
        }
    }

    fn handle_input_event(&mut self, event: InputEvent) {
        if let Some(ref mut recording) = self.recording {
            // Async screenshot capture - spawn in background thread immediately (no UI freeze!)
            let step_id = format!("step_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
            let step_id_clone = step_id.clone();
            let screenshots_dir = self.screenshots_dir.clone();
            let region_size = self.capture_region_size;
            let event_clone = event.clone();
            
            eprintln!("DEBUG: handle_input_event - Event type: {:?}, step_id: {}", event.event_type, step_id);
            
            // Clone sender for background thread
            if let Some(ref sender) = self.screenshot_sender {
                let sender_clone = sender.clone();
                
                // Spawn background thread for screenshot (doesn't block UI!)
                std::thread::spawn(move || {
                    eprintln!("DEBUG: Screenshot thread started for step_id: {}", step_id_clone);
                    let path_result = ScreenshotCapturer::capture_region(
                        &screenshots_dir,
                        &step_id_clone,
                        event_clone.x,
                        event_clone.y,
                        region_size,
                    );
                    
                    match path_result {
                        Ok(path) => {
                            eprintln!("DEBUG: Screenshot captured: {:?}, sending step_id: {}", path, step_id_clone);
                            if let Err(e) = sender_clone.send((path, step_id_clone.clone())) {
                                eprintln!("ERROR: Failed to send screenshot: {}", e);
                            } else {
                                eprintln!("DEBUG: Screenshot sent successfully for: {}", step_id_clone);
                            }
                        }
                        Err(e) => {
                            eprintln!("ERROR: Screenshot capture failed: {}", e);
                        }
                    }
                });
            } else {
                eprintln!("ERROR: screenshot_sender is None!");
            }
            
            // Store event temporarily - will be processed when screenshot is ready
            self.pending_steps.push((event, step_id.clone()));
            eprintln!("DEBUG: Added pending step. step_id: {}, total pending: {}", step_id, self.pending_steps.len());
        } else {
            eprintln!("WARN: handle_input_event called but recording is None!");
        }
    }
    
    fn process_screenshot_results(&mut self) {
        if let Some(ref receiver) = self.screenshot_receiver {
            while let Ok((screenshot_path, step_id)) = receiver.try_recv() {
                eprintln!("DEBUG: process_screenshot_results - Received screenshot: {:?}, step_id: {}", screenshot_path, step_id);
                eprintln!("DEBUG: Current pending steps: {:?}", self.pending_steps.iter().map(|(_, id)| id.clone()).collect::<Vec<_>>());
                
                // Find and remove matching pending step by step_id
                if let Some(pos) = self.pending_steps.iter().position(|(_, id)| id == &step_id) {
                    let (event, matched_id) = self.pending_steps.remove(pos);
                    
                    eprintln!("DEBUG: Found matching pending step! step_id: {}, event_type: {:?}", matched_id, event.event_type);
                    
                    if let Some(ref mut recording) = self.recording {
                        let step = Step::new(
                            event.event_type,
                            screenshot_path.clone(),
                            event.x,
                            event.y,
                            event.key,
                            event.window_title,
                        );

                        recording.steps.push(step);
                        self.selected_step_index = Some(recording.steps.len() - 1);

                        // Auto-save
                        if let Err(e) = self.storage.save_recording(recording) {
                            eprintln!("Failed to save recording: {}", e);
                        }
                        
                        eprintln!("DEBUG: Step created successfully! step_id: {}, screenshot: {:?}, Total steps: {}", 
                                 matched_id, screenshot_path, recording.steps.len());
                    } else {
                        eprintln!("ERROR: Recording is None when trying to create step!");
                    }
                } else {
                    eprintln!("WARN: No matching pending step found for step_id: {}", step_id);
                    eprintln!("DEBUG: Available pending step_ids: {:?}", 
                             self.pending_steps.iter().map(|(_, id)| id.clone()).collect::<Vec<_>>());
                }
            }
        } else {
            eprintln!("WARN: screenshot_receiver is None!");
        }
    }

    fn generate_ai_instruction(&mut self) {
        if self.generating_ai {
            return;
        }

        if let Some(ref recording) = self.recording {
            if let Some(index) = self.selected_step_index {
                if index < recording.steps.len() {
                    let step = &recording.steps[index];
                    let screenshot_path = step.screenshot_path.clone();

                    if screenshot_path.exists() {
                        self.generating_ai = true;
                        let llm_client = LLMClient::new(self.llm_client.model().to_string());
                        let screenshot_path_clone = screenshot_path.clone();
                        let context = format!(
                            "Step: {} - Event: {:?}",
                            step.title,
                            step.event_type
                        );
                        let sender = self.ai_result_sender.clone();
                        let step_index = index;

                        std::thread::spawn(move || {
                            let rt = tokio::runtime::Runtime::new().unwrap();
                            if let Ok(instruction) = rt.block_on(
                                llm_client.generate_instruction(&screenshot_path_clone, &context)
                            ) {
                                if let Some(ref s) = sender {
                                    let _ = s.send((step_index, instruction));
                                }
                            } else if let Some(ref s) = sender {
                                let _ = s.send((step_index, "Failed to generate instruction".to_string()));
                            }
                        });
                    }
                }
            }
        }
    }

    fn export_to_docx(&self) {
        if let Some(ref recording) = self.recording {
            let default_path = format!("{}.docx", recording.name);
            if let Some(path) = rfd::FileDialog::new()
                .set_file_name(&default_path)
                .save_file()
            {
                if let Err(e) = Exporter::export_to_docx(recording, &path) {
                    eprintln!("Failed to export DOCX: {}", e);
                }
            }
        }
    }

    fn export_to_pdf(&self) {
        if let Some(ref recording) = self.recording {
            let default_path = format!("{}.pdf", recording.name);
            if let Some(path) = rfd::FileDialog::new()
                .set_file_name(&default_path)
                .save_file()
            {
                if let Err(e) = Exporter::export_to_pdf(recording, &path) {
                    eprintln!("Failed to export PDF: {}", e);
                }
            }
        }
    }
}

impl eframe::App for CapturerApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Process input events
        self.process_input_events();
        
        // Process completed screenshots (async)
        self.process_screenshot_results();
        
        // Request repaint frequently to keep UI responsive
        ctx.request_repaint();

        // Top bar - use a different pattern to avoid borrow checker issues
        let is_recording = self.is_recording;
        let mut start_rec = false;
        let mut stop_rec = false;
        let mut export_docx = false;
        let mut export_pdf = false;
        
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if is_recording {
                    if ui.button("Stop Recording").clicked() {
                        stop_rec = true;
                    }
                    ui.label(RichText::new("● Recording...").color(Color32::RED));
                } else {
                    if ui.button("Start Recording").clicked() {
                        start_rec = true;
                    }
                }

                ui.separator();

                ui.menu_button("Export", |ui| {
                    if ui.button("Export to DOCX").clicked() {
                        export_docx = true;
                    }
                    if ui.button("Export to PDF").clicked() {
                        export_pdf = true;
                    }
                });
            });
        });
        
        // Floating overlay UI when recording (always visible, draggable)
        // Use a minimal collapsible window that shows pending count when collapsed
        if is_recording {
            let window_title = if self.overlay_collapsed {
                format!("🔴 {} pending", self.pending_steps.len())
            } else {
                "🔴 Recording Control".to_string()
            };
            
            let response = egui::Window::new(window_title)
                .collapsible(true)
                .title_bar(true)
                .resizable(false)
                .default_pos(self.overlay_pos)
                .show(ctx, |ui| {
                    if !self.overlay_collapsed {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("🔴 RECORDING").color(Color32::RED).strong());
                                if ui.small_button("−").clicked() {
                                    self.overlay_collapsed = true;
                                }
                            });
                            
                            ui.separator();
                            
                            ui.label("Capture Region Size:");
                            let region_size_text = format!("{}px", self.capture_region_size);
                            ui.add(egui::Slider::new(&mut self.capture_region_size, 200..=800)
                                .text(region_size_text));
                            
                            ui.separator();
                            
                            ui.horizontal(|ui| {
                                if ui.button("⏭ Skip Next").clicked() {
                                    // Skip the next pending step (oldest first)
                                    if !self.pending_steps.is_empty() {
                                        self.pending_steps.remove(0);
                                    }
                                }
                                if ui.button("Stop").clicked() {
                                    stop_rec = true;
                                }
                            });
                            
                            ui.label(format!("Pending: {} steps", self.pending_steps.len()));
                        });
                    } else {
                        // Collapsed view - just show expand button
                        ui.horizontal(|ui| {
                            ui.label(format!("{} pending", self.pending_steps.len()));
                            if ui.button("Expand").clicked() {
                                self.overlay_collapsed = false;
                            }
                        });
                    }
                });
                
            // Update overlay position if window was dragged
            if let Some(inner_response) = response {
                if inner_response.response.dragged() {
                    self.overlay_pos = inner_response.response.rect.min;
                }
            }
        }
        
        // Handle actions after UI is done
        if start_rec {
            self.start_recording();
        }
        if stop_rec {
            self.stop_recording();
        }
        if export_docx {
            self.export_to_docx();
        }
        if export_pdf {
            self.export_to_pdf();
        }

        // Main layout
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Left panel - Step list
                ui.vertical(|ui| {
                    ui.set_width(300.0);
                    ui.heading("Steps");
                    
                    if let Some(ref recording) = self.recording {
                        StepListView::show(
                            ui,
                            &recording.steps,
                            &mut self.selected_step_index,
                            &self.screenshots_dir,
                        );
                    } else {
                        ui.label("No recording started");
                    }
                });

                ui.separator();

                // Right panel - Step editor
                ui.vertical(|ui| {
                    ui.heading("Edit Step");

                    let mut should_generate_ai = false;
                    if let Some(ref mut recording) = self.recording {
                        let step = self.selected_step_index
                            .and_then(|i| recording.steps.get_mut(i));
                        
                        let ai_clicked = StepEditor::show(ui, step, !self.generating_ai);
                        if ai_clicked {
                            should_generate_ai = true;
                        }
                        
                        if self.generating_ai {
                            ui.label("Generating instruction...");
                        }
                        
                        // Delete button for selected step
                        if let Some(index) = self.selected_step_index {
                            if index < recording.steps.len() {
                                ui.separator();
                                if ui.button("🗑️ Delete This Step").clicked() {
                                    recording.steps.remove(index);
                                    self.selected_step_index = if recording.steps.is_empty() {
                                        None
                                    } else if index >= recording.steps.len() {
                                        Some(recording.steps.len() - 1)
                                    } else {
                                        Some(index)
                                    };
                                    if let Err(e) = self.storage.save_recording(recording) {
                                        eprintln!("Failed to save: {}", e);
                                    }
                                }
                            }
                        }
                    } else {
                        ui.label("Start a recording to begin");
                    }
                    
                    // Generate AI after borrow is released
                    if should_generate_ai {
                        self.generate_ai_instruction();
                    }
                });
            });
        });
    }
}

