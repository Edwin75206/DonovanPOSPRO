use crate::error::{AppError, Result, ensure};
use serde::Serialize;
use ttf_parser::{Face, GlyphId};

pub const REGULAR: &[u8] = include_bytes!("../../../../assets/fonts/DonovanPaper-Regular.ttf");
pub const BOLD: &[u8] = include_bytes!("../../../../assets/fonts/DonovanPaper-Bold.ttf");
pub const MM: f32 = 72.0 / 25.4;
pub const INK: [u8; 3] = [25, 29, 31];
pub const GRAY: [u8; 3] = [83, 91, 95];
pub const PALE: [u8; 3] = [236, 238, 239];

pub struct Fonts {
    regular: Face<'static>,
    bold: Face<'static>,
}
impl Fonts {
    pub fn load() -> Result<Self> {
        let parse = |data| {
            Face::parse(data, 0).map_err(|error| {
                AppError::internal(
                    "DOCUMENT_FONT",
                    "No pudimos preparar la tipografía del documento.",
                    error,
                )
            })
        };
        Ok(Self {
            regular: parse(REGULAR)?,
            bold: parse(BOLD)?,
        })
    }
    pub fn face(&self, bold: bool) -> &Face<'static> {
        if bold { &self.bold } else { &self.regular }
    }
    pub fn glyph(&self, c: char, bold: bool) -> GlyphId {
        self.face(bold)
            .glyph_index(c)
            .or_else(|| self.face(bold).glyph_index('?'))
            .unwrap_or(GlyphId(0))
    }
    pub fn width(&self, value: &str, size: f32, bold: bool) -> f32 {
        let face = self.face(bold);
        value
            .chars()
            .map(|c| face.glyph_hor_advance(self.glyph(c, bold)).unwrap_or(0) as f32)
            .sum::<f32>()
            * size
            / face.units_per_em() as f32
    }
    pub fn wrap(&self, text: &str, width: f32, size: f32, bold: bool) -> Vec<String> {
        let mut lines = Vec::new();
        for paragraph in text.replace('\r', "").split('\n') {
            let mut current = String::new();
            for word in paragraph.split_whitespace() {
                let candidate = if current.is_empty() {
                    word.to_owned()
                } else {
                    format!("{current} {word}")
                };
                if self.width(&candidate, size, bold) <= width {
                    current = candidate;
                    continue;
                }
                if !current.is_empty() {
                    lines.push(std::mem::take(&mut current));
                }
                for ch in word.chars() {
                    let mut candidate = current.clone();
                    candidate.push(ch);
                    if !current.is_empty() && self.width(&candidate, size, bold) > width {
                        lines.push(std::mem::take(&mut current));
                    }
                    current.push(ch);
                }
            }
            if !current.is_empty() {
                lines.push(current);
            } else if paragraph.is_empty() {
                lines.push(String::new());
            }
        }
        lines
    }
}
#[derive(Clone, Debug, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Draw {
    Text {
        x: f32,
        y: f32,
        text: String,
        size: f32,
        bold: bool,
        width: f32,
        color: [u8; 3],
    },
    Rect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: [u8; 3],
    },
    Logo {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub width: f32,
    pub height: f32,
    pub margin: f32,
    pub operations: Vec<Draw>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BusinessDocument {
    pub title: String,
    pub kind: String,
    pub pages: Vec<Page>,
    pub logo_data: Option<String>,
    pub warnings: Vec<String>,
}
pub struct Layout {
    pub fonts: Fonts,
    pub document: BusinessDocument,
    pub y: f32,
    pub width: f32,
    pub margin: f32,
    pub roll: bool,
    continued: String,
    page: Page,
    table_header: Option<(Vec<String>, Vec<f32>)>,
}
impl Layout {
    pub fn new(
        title: &str,
        kind: &str,
        width_mm: f32,
        roll: bool,
        logo: Option<String>,
    ) -> Result<Self> {
        let margin = if roll { 5.0 * MM } else { 14.0 * MM };
        let width = width_mm * MM;
        Ok(Self {
            fonts: Fonts::load()?,
            document: BusinessDocument {
                title: title.into(),
                kind: kind.into(),
                pages: vec![],
                logo_data: logo,
                warnings: vec![],
            },
            y: margin,
            width,
            margin,
            roll,
            continued: title.into(),
            page: Page {
                width,
                height: 297.0 * MM,
                margin,
                operations: vec![],
            },
            table_header: None,
        })
    }
    pub fn content_width(&self) -> f32 {
        self.width - self.margin * 2.0
    }
    pub fn landscape(title: &str, logo: Option<String>) -> Result<Self> {
        let mut paper = Self::new(title, "report", 297.0, false, logo)?;
        paper.page.height = 210.0 * MM;
        Ok(paper)
    }
    pub fn logo_at(&mut self, x: f32, y: f32, width: f32, height: f32) {
        if self.document.logo_data.is_some() {
            self.page.operations.push(Draw::Logo {
                x,
                y,
                width,
                height,
            });
        }
    }
    pub fn text_at(
        &mut self,
        x: f32,
        baseline: f32,
        text: &str,
        size: f32,
        bold: bool,
        color: [u8; 3],
    ) {
        let clean: String = text.chars().filter(|c| !c.is_control()).map(|c| {
            if self.fonts.face(bold).glyph_index(c).is_some() { c }
            else {
                if self.document.warnings.is_empty() { self.document.warnings.push("Algunos símbolos no están disponibles en la tipografía del papel y se muestran como ?. Los datos originales se conservan.".into()); }
                '?'
            }
        }).collect();
        let width = self.fonts.width(&clean, size, bold);
        self.page.operations.push(Draw::Text {
            x,
            y: baseline,
            text: clean,
            size,
            bold,
            width,
            color,
        });
    }
    pub fn rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [u8; 3]) {
        self.page.operations.push(Draw::Rect {
            x,
            y,
            width,
            height,
            color,
        });
    }
    pub fn space(&mut self, points: f32) {
        self.y += points;
    }
    pub fn ensure(&mut self, height: f32) {
        if self.y + height <= self.page.height - self.margin - 19.0 {
            return;
        }
        let bottom = self.page.height - self.margin - 9.0;
        self.text_at(
            self.margin,
            bottom,
            "Continúa en la siguiente página",
            7.0,
            false,
            GRAY,
        );
        let next = Page {
            width: self.width,
            height: self.page.height,
            margin: self.margin,
            operations: vec![],
        };
        self.document
            .pages
            .push(std::mem::replace(&mut self.page, next));
        self.y = self.margin;
        let continued = self.continued.clone();
        for line in self.fonts.wrap(&continued, self.content_width(), 8.0, true) {
            self.text_at(self.margin, self.y + 8.0, &line, 8.0, true, INK);
            self.y += 11.0;
        }
        self.rule();
        if let Some((cells, fractions)) = self.table_header.clone() {
            self.table_row(&cells, &fractions, true);
        }
    }
    pub fn paragraph(&mut self, text: &str, size: f32, bold: bool, center: bool, color: [u8; 3]) {
        for line in self.fonts.wrap(text, self.content_width(), size, bold) {
            self.ensure(size * 1.5);
            let x = if center {
                (self.width - self.fonts.width(&line, size, bold)) / 2.0
            } else {
                self.margin
            };
            self.text_at(x, self.y + size, &line, size, bold, color);
            self.y += size * 1.45;
        }
    }
    pub fn pair(&mut self, label: &str, value: &str, size: f32, bold: bool) {
        let value_width = self.fonts.width(value, size, bold);
        let size = if value.contains('$') && value_width > self.content_width() {
            size * self.content_width() / value_width
        } else {
            size
        };
        let value_width = self.fonts.width(value, size, bold);
        let left_width = self.content_width() - value_width - 8.0;
        if left_width < self.content_width() * 0.35 || value_width > self.content_width() {
            self.paragraph(label, 8.0, false, false, GRAY);
            self.paragraph(value, size, bold, false, INK);
            return;
        }
        let labels = self.fonts.wrap(label, left_width, size, bold);
        self.ensure(labels.len().max(1) as f32 * size * 1.45);
        self.text_at(
            self.width - self.margin - value_width,
            self.y + size,
            value,
            size,
            bold,
            INK,
        );
        for line in labels {
            self.text_at(self.margin, self.y + size, &line, size, bold, INK);
            self.y += size * 1.45;
        }
    }
    pub fn rule(&mut self) {
        self.space(5.0);
        self.ensure(8.0);
        self.rect(self.margin, self.y, self.content_width(), 0.6, PALE);
        self.space(7.0);
    }
    pub fn logo(&mut self, aspect: f32) {
        if self.document.logo_data.is_none() {
            return;
        }
        let width = self
            .content_width()
            .min(if self.roll { 48.0 * MM } else { 42.0 * MM });
        let height = (width / aspect).min(39.0 * MM);
        let width = height * aspect;
        self.ensure(height + 7.0);
        self.page.operations.push(Draw::Logo {
            x: (self.width - width) / 2.0,
            y: self.y,
            width,
            height,
        });
        self.y += height + 7.0;
    }
    pub fn table_row(&mut self, cells: &[String], fractions: &[f32], header: bool) {
        if header {
            self.table_header = None;
        }
        let size = if self.roll { 7.5 } else { 9.0 };
        let width = self.content_width();
        let lines: Vec<Vec<String>> = cells
            .iter()
            .zip(fractions)
            .map(|(cell, fraction)| self.fonts.wrap(cell, width * fraction - 10.0, size, header))
            .collect();
        let height =
            lines.iter().map(Vec::len).max().unwrap_or(1).max(1) as f32 * size * 1.45 + 12.0;
        self.ensure(height);
        if header {
            self.table_header = Some((cells.to_vec(), fractions.to_vec()));
            self.rect(self.margin, self.y, width, height, PALE);
        }
        let mut x = self.margin + 5.0;
        for (index, column) in lines.iter().enumerate() {
            for (line_index, line) in column.iter().enumerate() {
                self.text_at(
                    x,
                    self.y + 6.0 + size + line_index as f32 * size * 1.45,
                    line,
                    size,
                    header,
                    INK,
                );
            }
            x += fractions[index] * width;
        }
        self.y += height;
        if !header {
            self.rect(self.margin, self.y, width, 0.4, PALE);
        }
    }
    pub fn end_table(&mut self) {
        self.table_header = None;
    }
    pub fn finish(mut self) -> Result<BusinessDocument> {
        if self.roll {
            self.page.height = (self.y + self.margin + 18.0).min(297.0 * MM).max(50.0 * MM);
        }
        self.document.pages.push(self.page);
        ensure(
            self.document.pages.len() <= 100,
            "DOCUMENT_LIMIT",
            "El documento supera 100 páginas. Reduce el periodo o la selección para imprimir.",
        )?;
        let count = self.document.pages.len();
        for (index, page) in self.document.pages.iter_mut().enumerate() {
            let footer = format!("Donovan POS Pro  ·  {} / {}", index + 1, count);
            let width = self.fonts.width(&footer, 6.5, false);
            page.operations.push(Draw::Text {
                x: (page.width - width) / 2.0,
                y: page.height - page.margin + 3.0,
                text: footer,
                size: 6.5,
                bold: false,
                width,
                color: GRAY,
            });
        }
        Ok(self.document)
    }
}
pub fn money(cents: i64) -> String {
    let absolute = (cents as i128).abs();
    let digits = (absolute / 100).to_string();
    let mut grouped = String::new();
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    format!(
        "{}${grouped}.{:02}",
        if cents < 0 { "−" } else { "" },
        absolute % 100
    )
}
pub fn quantity(units: i64) -> String {
    let absolute = (units as i128).abs();
    let value = format!(
        "{}{}.{:04}",
        if units < 0 { "−" } else { "" },
        absolute / 10000,
        absolute % 10000
    );
    value.trim_end_matches('0').trim_end_matches('.').into()
}
pub fn unit(unit: i64) -> &'static str {
    ["pza", "m", "kg", "L", "caja", "rollo"]
        .get(unit as usize)
        .copied()
        .unwrap_or("")
}
