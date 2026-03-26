use anyhow::Result;
use crate::models::Recording;
use std::path::PathBuf;
use docx_rs::*;
use printpdf::*;
use std::fs::File;
use std::io::BufWriter;

pub struct Exporter;

impl Exporter {
    pub fn export_to_docx(recording: &Recording, output_path: &PathBuf) -> Result<()> {
        let mut doc = Docx::new();

        // Cover page
        let cover = Paragraph::new()
            .add_run(Run::new().add_text(&recording.name).bold().size(32))
            .align(AlignmentType::Center);
        doc = doc.add_paragraph(cover);

        // Add spacing
        doc = doc.add_paragraph(Paragraph::new());

        // Created date
        let date = chrono::DateTime::from_timestamp(recording.created_at, 0)
            .unwrap_or_default()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        let date_para = Paragraph::new()
            .add_run(Run::new().add_text(format!("Created: {}", date)).size(16))
            .align(AlignmentType::Center);
        doc = doc.add_paragraph(date_para);

        // Page break
        doc = doc.add_paragraph(Paragraph::new().page_break_before(true));

        // Table of Contents
        let toc_title = Paragraph::new()
            .add_run(Run::new().add_text("Table of Contents").bold().size(24));
        doc = doc.add_paragraph(toc_title);

        for (index, step) in recording.steps.iter().enumerate() {
            let toc_entry = Paragraph::new()
                .add_run(Run::new().add_text(format!("Step {}: {}", index + 1, step.title)).size(12));
            doc = doc.add_paragraph(toc_entry);
        }

        // Page break
        doc = doc.add_paragraph(Paragraph::new().page_break_before(true));

        // Steps
        for (index, step) in recording.steps.iter().enumerate() {
            // Step title
            let step_title = Paragraph::new()
                .add_run(Run::new().add_text(format!("Step {}: {}", index + 1, step.title)).bold().size(20))
                .page_break_before(index > 0);
            doc = doc.add_paragraph(step_title);

            // Step description
            if !step.description.is_empty() {
                let step_desc = Paragraph::new()
                    .add_run(Run::new().add_text(&step.description).size(12));
                doc = doc.add_paragraph(step_desc);
            }

            // Screenshot (if exists) - Note: docx-rs image support is limited
            // For now, we'll skip images in DOCX or add them differently
            if step.screenshot_path.exists() {
                // Image insertion in docx-rs requires Pic struct creation
                // Skipping for now - can be enhanced later
            }

            doc = doc.add_paragraph(Paragraph::new());
        }

        // Save document
        let mut file = File::create(output_path)?;
        doc.build().pack(&mut file)?;

        Ok(())
    }

    pub fn export_to_pdf(recording: &Recording, output_path: &PathBuf) -> Result<()> {
        let (doc, page1, layer1) = PdfDocument::new("Recording", Mm(210.0), Mm(297.0), "Layer 1");
        let current_layer = doc.get_page(page1).get_layer(layer1);

        // Set up fonts
        let font = doc.add_builtin_font(BuiltinFont::Helvetica)?;
        let font_bold = doc.add_builtin_font(BuiltinFont::HelveticaBold)?;

        let mut y_position = 250.0; // Start from top

        // Cover page
        current_layer.use_text(
            &recording.name,
            32.0,
            Mm(105.0),
            Mm(y_position),
            &font_bold,
        );
        y_position -= 20.0;

        let date = chrono::DateTime::from_timestamp(recording.created_at, 0)
            .unwrap_or_default()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        current_layer.use_text(
            &format!("Created: {}", date),
            16.0,
            Mm(105.0),
            Mm(y_position),
            &font,
        );

        // Add new page for TOC
        let (page2, layer2) = doc.add_page(Mm(210.0), Mm(297.0), "Layer 1");
        let toc_layer = doc.get_page(page2).get_layer(layer2);
        
        let mut toc_y = 270.0;
        toc_layer.use_text("Table of Contents", 24.0, Mm(10.0), Mm(toc_y), &font_bold);
        toc_y -= 15.0;

        for (index, step) in recording.steps.iter().enumerate() {
            toc_layer.use_text(
                &format!("Step {}: {}", index + 1, step.title),
                12.0,
                Mm(15.0),
                Mm(toc_y),
                &font,
            );
            toc_y -= 8.0;
            if toc_y < 20.0 {
                // Add new page if needed
                let (new_page, new_layer) = doc.add_page(Mm(210.0), Mm(297.0), "Layer 1");
                toc_y = 280.0;
                let _ = doc.get_page(new_page).get_layer(new_layer);
            }
        }

        // Steps pages
        for (index, step) in recording.steps.iter().enumerate() {
            let (page, layer) = if index == 0 {
                doc.add_page(Mm(210.0), Mm(297.0), "Layer 1")
            } else {
                doc.add_page(Mm(210.0), Mm(297.0), "Layer 1")
            };
            let step_layer = doc.get_page(page).get_layer(layer);

            let mut y_pos = 270.0;

            // Step title
            step_layer.use_text(
                &format!("Step {}: {}", index + 1, step.title),
                20.0,
                Mm(10.0),
                Mm(y_pos),
                &font_bold,
            );
            y_pos -= 15.0;

            // Step description
            if !step.description.is_empty() {
                let words: Vec<&str> = step.description.split_whitespace().collect();
                let mut line = String::new();
                let mut line_y = y_pos;

                for word in words {
                    let test_line = if line.is_empty() {
                        word.to_string()
                    } else {
                        format!("{} {}", line, word)
                    };

                    if test_line.len() > 80 {
                        if !line.is_empty() {
                            step_layer.use_text(&line, 12.0, Mm(10.0), Mm(line_y), &font);
                            line_y -= 8.0;
                            line = word.to_string();
                        }
                    } else {
                        line = test_line;
                    }

                    if line_y < 50.0 {
                        // Start new page
                        let (new_page, new_layer) = doc.add_page(Mm(210.0), Mm(297.0), "Layer 1");
                        line_y = 270.0;
                        let _ = doc.get_page(new_page).get_layer(new_layer);
                    }
                }

                if !line.is_empty() {
                    step_layer.use_text(&line, 12.0, Mm(10.0), Mm(line_y), &font);
                    y_pos = line_y - 15.0;
                }
            }

            // Screenshot - Simplified PDF export without images for now
            // Full image support requires more complex printpdf API usage
            if step.screenshot_path.exists() {
                step_layer.use_text(
                    &format!("[Screenshot: {}]", step.screenshot_path.display()),
                    10.0,
                    Mm(10.0),
                    Mm(y_pos),
                    &font,
                );
                y_pos -= 10.0;
            }
        }

        // Save PDF
        doc.save(&mut BufWriter::new(File::create(output_path)?))?;

        Ok(())
    }
}

