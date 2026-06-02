use anyhow::Result;
use eframe::egui;
use egui_commonmark::*;
use poll_promise::Promise;
use scraper::{Html, Selector};
use std::collections::HashSet;
use regex::Regex;

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct SepEntry {
    pub title: String,
    pub url: String,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct HighlightedSnippet {
    pub entry_title: String,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct Section {
    pub id: String,
    pub title: String,
    pub body: String,
}

pub struct Scraper {
    client: reqwest::Client,
}

impl Scraper {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
        }
    }

    pub async fn get_entries(&self) -> Result<Vec<SepEntry>> {
        let url = "https://plato.stanford.edu/contents.html";
        let response = self.client.get(url).send().await?.text().await?;
        let document = Html::parse_document(&response);
        let content_selector = Selector::parse("#content").unwrap();
        let a_selector = Selector::parse("a").unwrap();

        let mut entries = Vec::new();
        let mut seen_urls = HashSet::new();

        if let Some(content) = document.select(&content_selector).next() {
            for a in content.select(&a_selector) {
                if let Some(href) = a.value().attr("href") {
                    if href.contains("entries/") {
                        let full_url = if !href.starts_with("http") {
                            format!("https://plato.stanford.edu/{}", href)
                        } else {
                            href.to_string()
                        };

                        let title = a.text().collect::<Vec<_>>().join("").trim().to_string();
                        if !title.is_empty() && seen_urls.insert(full_url.clone()) {
                            entries.push(SepEntry { title, url: full_url });
                        }
                    }
                }
            }
        }
        Ok(entries)
    }

    pub async fn get_content(&self, url: String) -> Result<Vec<Section>> {
        let response = self.client.get(&url).send().await?.text().await?;
        let document = Html::parse_document(&response);
        let main_selector = Selector::parse("#main-content, #content").unwrap();
        
        let citation_regex = Regex::new(r"\[\s*\d+\s*\]").unwrap();
        let skip_boilerplate = vec![
            "Bibliography", "Academic Tools", "Friends PDF Preview", 
            "Author and Citation Info", "Back to Top", "Other Internet Resources", "Related Entries"
        ];

        let mut sections = Vec::new();
        let mut current_section = Section { id: "intro".to_string(), title: "Introduction".to_string(), body: String::new() };

        if let Some(content) = document.select(&main_selector).next() {
            let text_selector = Selector::parse("p, h1, h2, h3, h4, blockquote, ul > li, ol > li").unwrap();
            
            for element in content.select(&text_selector) {
                let tag = element.value().name();
                let mut text = String::new();
                for node in element.children() {
                    let node_text = scraper::ElementRef::wrap(node).map(|e| e.text().collect::<String>()).unwrap_or_default();
                    let mut handled = false;
                    if let Some(el) = node.value().as_element() {
                        if el.name() == "a" {
                            if let Some(href) = el.attr("href") {
                                if href.starts_with('#') {
                                    // Standard Markdown internal link
                                    text.push_str(&format!("[{}](#{})", node_text, &href[1..]));
                                    handled = true;
                                }
                            }
                        }
                    }
                    if !handled { text.push_str(&node_text); }
                }
                let mut clean_text = text.split_whitespace().collect::<Vec<_>>().join(" ");
                
                if clean_text.is_empty() { continue; }
                if skip_boilerplate.iter().any(|&h| clean_text.contains(h)) { continue; }
                clean_text = citation_regex.replace_all(&clean_text, "").to_string();

                if tag.starts_with('h') {
                    if !current_section.body.is_empty() {
                        sections.push(current_section.clone());
                    }
                    let id = element.value().attr("id").map(|s| s.to_string())
                        .unwrap_or_else(|| clean_text.to_lowercase().replace(' ', "-"));
                    
                    current_section = Section {
                        id,
                        title: clean_text.clone(),
                        body: format!("# {}\n\n", clean_text),
                    };
                } else {
                    match tag {
                        "blockquote" => current_section.body.push_str(&format!("> {}\n\n", clean_text)),
                        "li" => current_section.body.push_str(&format!("* {}\n", clean_text)),
                        _ => current_section.body.push_str(&format!("{}\n\n", clean_text)),
                    }
                }
            }
            sections.push(current_section);
        }
        
        Ok(sections)
    }
}

pub struct StoaApp {
    entries: Option<Promise<Result<Vec<SepEntry>>>>,
    current_sections: Option<Promise<Result<Vec<Section>>>>,
    selected_url: String,
    selected_title: String,
    filter: String,
    show_sidebar: bool,
    scraper: std::sync::Arc<Scraper>,
    cache: CommonMarkCache,
    read_entries: HashSet<String>,
    highlights: Vec<HighlightedSnippet>,
    scroll_to_id: Option<String>,
}

impl StoaApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let scraper = std::sync::Arc::new(Scraper::new());
        let s_clone = scraper.clone();
        
        let mut fonts = egui::FontDefinitions::default();
        if let Some(mono_fonts) = fonts.families.get(&egui::FontFamily::Monospace) {
            let mono_name = mono_fonts[0].clone();
            fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap()
                .insert(0, mono_name);
        }
        cc.egui_ctx.set_fonts(fonts);

        let mut style = (*cc.egui_ctx.global_style()).clone();
        style.text_styles.insert(egui::TextStyle::Heading, egui::FontId::new(70.0, egui::FontFamily::Proportional));
        style.text_styles.insert(egui::TextStyle::Body, egui::FontId::new(28.0, egui::FontFamily::Proportional));
        style.visuals.override_text_color = Some(egui::Color32::WHITE);
        cc.egui_ctx.set_global_style(style);

        Self {
            entries: Some(Promise::spawn_async(async move { s_clone.get_entries().await })),
            current_sections: None,
            selected_url: String::new(),
            selected_title: "Stoa".to_string(),
            filter: String::new(),
            show_sidebar: true,
            scraper,
            cache: CommonMarkCache::default(),
            read_entries: HashSet::new(),
            highlights: Vec::new(),
            scroll_to_id: None,
        }
    }
}

impl eframe::App for StoaApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx();
        
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = egui::Color32::from_rgba_unmultiplied(0, 0, 0, 0); 
        visuals.window_fill = egui::Color32::from_rgb(10, 10, 10);
        visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        visuals.override_text_color = Some(egui::Color32::WHITE);
        ctx.set_visuals(visuals);

        if ui.input(|i| i.key_pressed(egui::Key::M)) {
            self.show_sidebar = !self.show_sidebar;
        }

        if self.show_sidebar {
            egui::Panel::left("sidebar")
                .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(5, 5, 5)).inner_margin(20))
                .resizable(false)
                .default_size(350.0)
                .show_inside(ui, |ui| {
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new("S T O A").size(24.0).strong().color(egui::Color32::from_rgb(153, 122, 255)));
                    ui.add_space(20.0);

                    ui.add(egui::TextEdit::singleline(&mut self.filter)
                        .hint_text("Search...")
                        .font(egui::FontId::proportional(16.0))
                        .desired_width(f32::INFINITY));
                    ui.add_space(20.0);

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        if let Some(promise) = &self.current_sections {
                            if let Some(Ok(sections)) = promise.ready() {
                                ui.label(egui::RichText::new("OUTLINE").size(11.0).strong().color(egui::Color32::from_gray(150)));
                                ui.add_space(5.0);
                                for section in sections {
                                    if ui.selectable_label(self.scroll_to_id.as_ref() == Some(&section.id), 
                                        egui::RichText::new(&section.title).size(14.0).color(egui::Color32::WHITE)).clicked() {
                                        self.scroll_to_id = Some(section.id.clone());
                                    }
                                }
                                ui.add_space(15.0);
                                ui.separator();
                                ui.add_space(15.0);
                            }
                        }

                        ui.label(egui::RichText::new("LIBRARY").size(11.0).strong().color(egui::Color32::from_gray(150)));
                        ui.add_space(5.0);
                        if let Some(promise) = &self.entries {
                            if let Some(Ok(entries)) = promise.ready() {
                                for entry in entries.iter().filter(|e| e.title.to_lowercase().contains(&self.filter.to_lowercase())) {
                                    let is_read = self.read_entries.contains(&entry.url);
                                    let text = egui::RichText::new(&entry.title).size(15.0)
                                        .color(if is_read { egui::Color32::from_gray(100) } else { egui::Color32::WHITE });

                                    if ui.selectable_label(self.selected_url == entry.url, text).clicked() {
                                        let url = entry.url.clone();
                                        let s = self.scraper.clone();
                                        self.selected_url = url.clone();
                                        self.selected_title = entry.title.clone();
                                        self.current_sections = Some(Promise::spawn_async(async move { s.get_content(url).await }));
                                        self.read_entries.insert(entry.url.clone());
                                        self.scroll_to_id = None;
                                    }
                                }
                            }
                        }
                    });
                });
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 100)))
            .show_inside(ui, |ui| {
                ui.add_space(10.0);
                if !self.show_sidebar {
                    if ui.button(egui::RichText::new("☰").size(24.0).color(egui::Color32::WHITE)).clicked() {
                        self.show_sidebar = true;
                    }
                }

                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .id_salt("main_scroll")
                    .show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(40.0);
                            let reading_width = (ui.available_width() * 0.85).min(750.0);
                            ui.set_max_width(reading_width);

                            if let Some(promise) = &self.current_sections {
                                match promise.ready() {
                                    None => { ui.add_space(200.0); ui.spinner(); }
                                    Some(Ok(sections)) => {
                                        for section in sections {
                                            let response = ui.vertical(|ui| {
                                                CommonMarkViewer::new()
                                                    .show(ui, &mut self.cache, &section.body);
                                            }).response;

                                            // Manual Jump detection
                                            if let Some(id) = ui.memory(|m| m.data.get_temp::<String>(ui.id().with("jump"))) {
                                                self.scroll_to_id = Some(id);
                                                ui.memory_mut(|m| m.data.remove::<String>(ui.id().with("jump")));
                                            }

                                            if let Some(target_id) = &self.scroll_to_id {
                                                if &section.id == target_id {
                                                    response.scroll_to_me(Some(egui::Align::TOP));
                                                }
                                            }
                                            ui.add_space(20.0);
                                        }
                                        if self.scroll_to_id.is_some() { self.scroll_to_id = None; }
                                        ui.add_space(400.0);
                                    }
                                    Some(Err(e)) => { ui.label(format!("Error: {}", e)); }
                                }
                            } else {
                                ui.add_space(200.0);
                                ui.label(egui::RichText::new("S T O A").size(100.0).strong().color(egui::Color32::WHITE).extra_letter_spacing(15.0));
                                ui.add_space(20.0);
                                ui.label(egui::RichText::new("A Zen sanctuary for thought.").italics().size(22.0).color(egui::Color32::WHITE));
                            }
                        });
                    });
            });
    }
}

fn main() -> eframe::Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime");
    let _guard = rt.enter();

    unsafe {
        std::env::set_var("WINIT_UNIX_BACKEND", "x11");
        std::env::remove_var("WAYLAND_DISPLAY");
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_transparent(true)
            .with_inner_size([1100.0, 1000.0])
            .with_decorations(true),
        ..Default::default()
    };

    eframe::run_native(
        "Stoa Zen",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(StoaApp::new(cc)))
        }),
    )
}
