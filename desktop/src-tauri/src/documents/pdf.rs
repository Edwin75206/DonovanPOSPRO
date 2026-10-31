use super::layout::{BOLD, BusinessDocument, Draw, Fonts, REGULAR};
use crate::error::{AppError, Result};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use lopdf::{
    Document, Object, ObjectId, Stream, StringFormat,
    content::{Content, Operation},
    dictionary,
};
use std::collections::{BTreeMap, BTreeSet};

fn pdf_error(error: impl std::fmt::Display) -> AppError {
    AppError::internal(
        "DOCUMENT_PDF",
        "No pudimos generar el PDF. La operación del negocio ya guardada se conserva.",
        error,
    )
}
fn unicode_string(value: &str) -> Object {
    let mut bytes = vec![0xfe, 0xff];
    for word in value.encode_utf16() {
        bytes.extend_from_slice(&word.to_be_bytes());
    }
    Object::String(bytes, StringFormat::Hexadecimal)
}
fn embed_font(
    document: &mut Document,
    fonts: &Fonts,
    bold: bool,
    used: &BTreeSet<char>,
) -> Result<(ObjectId, BTreeMap<char, u16>)> {
    let data = if bold { BOLD } else { REGULAR };
    let name = if bold {
        "DonovanPaper-Bold"
    } else {
        "DonovanPaper-Regular"
    };
    let face = fonts.face(bold);
    let scale = 1000.0 / face.units_per_em() as f32;
    let bbox = face.global_bounding_box();
    let file_id = document.add_object(Stream::new(
        dictionary! {"Length1"=>data.len() as i64},
        data.to_vec(),
    ));
    let descriptor=document.add_object(dictionary!{"Type"=>"FontDescriptor","FontName"=>name,"Flags"=>32,"FontBBox"=>vec![Object::Real(bbox.x_min as f32*scale),Object::Real(bbox.y_min as f32*scale),Object::Real(bbox.x_max as f32*scale),Object::Real(bbox.y_max as f32*scale)],"ItalicAngle"=>0,"Ascent"=>face.ascender() as f32*scale,"Descent"=>face.descender() as f32*scale,"CapHeight"=>face.capital_height().unwrap_or(face.ascender()) as f32*scale,"StemV"=>if bold {110} else {80},"FontFile2"=>file_id});
    let codes: BTreeMap<char, u16> = used
        .iter()
        .enumerate()
        .map(|(index, ch)| (*ch, (index + 1) as u16))
        .collect();
    let mut gids = vec![0u8; (codes.len() + 1) * 2];
    let mut widths = vec![Object::Integer(0), Object::Array(vec![Object::Integer(0)])];
    let mut mappings = Vec::new();
    for (&ch, &cid) in &codes {
        let glyph = fonts.glyph(ch, bold);
        gids[cid as usize * 2..cid as usize * 2 + 2].copy_from_slice(&glyph.0.to_be_bytes());
        widths.push(Object::Integer(cid as i64));
        widths.push(Object::Array(vec![Object::Real(
            face.glyph_hor_advance(glyph).unwrap_or(0) as f32 * scale,
        )]));
        let utf16: String = ch
            .encode_utf16(&mut [0u16; 2])
            .iter()
            .map(|word| format!("{word:04X}"))
            .collect();
        mappings.push(format!("<{cid:04X}> <{utf16}>"));
    }
    let gid_map = document.add_object(Stream::new(dictionary! {}, gids));
    let mut cmap = String::from(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n/CMapName /DonovanUnicode def\n/CMapType 2 def\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n",
    );
    for chunk in mappings.chunks(100) {
        cmap.push_str(&format!(
            "{} beginbfchar\n{}\nendbfchar\n",
            chunk.len(),
            chunk.join("\n")
        ));
    }
    cmap.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend");
    let unicode = document.add_object(Stream::new(dictionary! {}, cmap.into_bytes()));
    let descendant=document.add_object(dictionary!{"Type"=>"Font","Subtype"=>"CIDFontType2","BaseFont"=>name,"CIDSystemInfo"=>dictionary!{"Registry"=>Object::string_literal("Adobe"),"Ordering"=>Object::string_literal("Identity"),"Supplement"=>0},"FontDescriptor"=>descriptor,"CIDToGIDMap"=>gid_map,"DW"=>1000,"W"=>Object::Array(widths)});
    let font=document.add_object(dictionary!{"Type"=>"Font","Subtype"=>"Type0","BaseFont"=>name,"Encoding"=>"Identity-H","DescendantFonts"=>vec![Object::Reference(descendant)],"ToUnicode"=>unicode});
    Ok((font, codes))
}
fn embed_logo(document: &mut Document, encoded: &str) -> Result<ObjectId> {
    let bytes = STANDARD.decode(encoded).map_err(pdf_error)?;
    crate::appearance::validate_image(&bytes)?;
    let decoded = image::load_from_memory(&bytes).map_err(pdf_error)?;
    let image = if decoded.width() > 1600 || decoded.height() > 1600 {
        decoded.thumbnail(1600, 1600)
    } else {
        decoded
    }
    .to_rgba8();
    let mut color = Vec::with_capacity(image.len() / 4 * 3);
    let mut alpha = Vec::with_capacity(image.len() / 4);
    for pixel in image.pixels() {
        color.extend_from_slice(&pixel.0[..3]);
        alpha.push(pixel[3]);
    }
    let mask=document.add_object(Stream::new(dictionary!{"Type"=>"XObject","Subtype"=>"Image","Width"=>image.width() as i64,"Height"=>image.height() as i64,"ColorSpace"=>"DeviceGray","BitsPerComponent"=>8},alpha));
    Ok(document.add_object(Stream::new(dictionary!{"Type"=>"XObject","Subtype"=>"Image","Width"=>image.width() as i64,"Height"=>image.height() as i64,"ColorSpace"=>"DeviceRGB","BitsPerComponent"=>8,"SMask"=>mask},color)))
}
pub fn encode(source: &BusinessDocument) -> Result<Vec<u8>> {
    let mut document = Document::with_version("1.7");
    let fonts = Fonts::load()?;
    let mut normal = BTreeSet::new();
    let mut bold = BTreeSet::new();
    for page in &source.pages {
        for op in &page.operations {
            if let Draw::Text {
                text, bold: strong, ..
            } = op
            {
                if *strong {
                    bold.extend(text.chars());
                } else {
                    normal.extend(text.chars());
                }
            }
        }
    }
    let (regular_id, regular_codes) = embed_font(&mut document, &fonts, false, &normal)?;
    let (bold_id, bold_codes) = embed_font(&mut document, &fonts, true, &bold)?;
    let logo = source
        .logo_data
        .as_deref()
        .map(|data| embed_logo(&mut document, data))
        .transpose()?;
    let pages_id = document.new_object_id();
    let mut xobjects = lopdf::Dictionary::new();
    if let Some(logo) = logo {
        xobjects.set("Logo", logo);
    }
    let resources=document.add_object(dictionary!{"Font"=>dictionary!{"Regular"=>regular_id,"Bold"=>bold_id},"XObject"=>xobjects});
    let mut page_ids = vec![];
    for page in &source.pages {
        let mut operations = vec![];
        for op in &page.operations {
            match op {
                Draw::Text {
                    x,
                    y,
                    text,
                    size,
                    bold,
                    color,
                    ..
                } => {
                    let codes = if *bold { &bold_codes } else { &regular_codes };
                    let encoded: Vec<u8> = text
                        .chars()
                        .flat_map(|c| codes.get(&c).copied().unwrap_or(0).to_be_bytes())
                        .collect();
                    operations.push(Operation::new("BT", vec![]));
                    operations.push(Operation::new(
                        "rg",
                        color
                            .iter()
                            .map(|value| Object::Real(*value as f32 / 255.0))
                            .collect(),
                    ));
                    operations.push(Operation::new(
                        "Tf",
                        vec![
                            Object::Name(if *bold {
                                b"Bold".to_vec()
                            } else {
                                b"Regular".to_vec()
                            }),
                            Object::Real(*size),
                        ],
                    ));
                    operations.push(Operation::new(
                        "Tm",
                        vec![
                            1.into(),
                            0.into(),
                            0.into(),
                            1.into(),
                            Object::Real(*x),
                            Object::Real(page.height - y),
                        ],
                    ));
                    operations.push(Operation::new(
                        "Tj",
                        vec![Object::String(encoded, StringFormat::Hexadecimal)],
                    ));
                    operations.push(Operation::new("ET", vec![]));
                }
                Draw::Rect {
                    x,
                    y,
                    width,
                    height,
                    color,
                } => {
                    operations.push(Operation::new(
                        "rg",
                        color
                            .iter()
                            .map(|value| Object::Real(*value as f32 / 255.0))
                            .collect(),
                    ));
                    operations.push(Operation::new(
                        "re",
                        vec![
                            Object::Real(*x),
                            Object::Real(page.height - y - height),
                            Object::Real(*width),
                            Object::Real(*height),
                        ],
                    ));
                    operations.push(Operation::new("f", vec![]));
                }
                Draw::Logo {
                    x,
                    y,
                    width,
                    height,
                } if logo.is_some() => {
                    operations.push(Operation::new("q", vec![]));
                    operations.push(Operation::new(
                        "cm",
                        vec![
                            Object::Real(*width),
                            0.into(),
                            0.into(),
                            Object::Real(*height),
                            Object::Real(*x),
                            Object::Real(page.height - y - height),
                        ],
                    ));
                    operations.push(Operation::new("Do", vec![Object::Name(b"Logo".to_vec())]));
                    operations.push(Operation::new("Q", vec![]));
                }
                _ => {}
            }
        }
        let content = Content { operations };
        let content_id = document.add_object(Stream::new(
            dictionary! {},
            content.encode().map_err(pdf_error)?,
        ));
        let page_id=document.add_object(dictionary!{"Type"=>"Page","Parent"=>pages_id,"Contents"=>content_id,"Resources"=>resources,"MediaBox"=>vec![0.into(),0.into(),Object::Real(page.width),Object::Real(page.height)]});
        page_ids.push(Object::Reference(page_id));
    }
    document.objects.insert(
        pages_id,
        Object::Dictionary(
            dictionary! {"Type"=>"Pages","Count"=>page_ids.len() as i64,"Kids"=>page_ids},
        ),
    );
    let catalog = document.add_object(
        dictionary! {"Type"=>"Catalog","Pages"=>pages_id,"Lang"=>Object::string_literal("es-MX")},
    );
    let info=document.add_object(dictionary!{"Title"=>unicode_string(&source.title),"Author"=>Object::string_literal("Donovan POS Pro"),"Creator"=>Object::string_literal("Donovan Systems"),"Producer"=>Object::string_literal("Donovan POS Pro / Rust / lopdf")});
    document.trailer.set("Root", catalog);
    document.trailer.set("Info", info);
    document.compress();
    let mut bytes = vec![];
    document.save_to(&mut bytes).map_err(pdf_error)?;
    Ok(bytes)
}
