pub mod bilan_report;
pub mod plan_action;
pub mod stock_report;

use rust_xlsxwriter::{
    ConditionalFormatCell, ConditionalFormatCellRule, Format, FormatBorder, Worksheet, XlsxError,
};

/// En-tête bleu #2C5F8A, texte blanc, gras, bordure fine (RG-052)
pub fn create_header_format() -> Format {
    Format::new()
        .set_bold()
        .set_background_color("2C5F8A")
        .set_font_color("FFFFFF")
        .set_font_size(11)
        .set_border(FormatBorder::Thin)
        .set_text_wrap()
}

/// Format date dd/mm/yyyy (RG-050)
pub fn create_date_format() -> Format {
    Format::new().set_num_format("dd/mm/yyyy")
}

/// Format nombre décimal #,##0.00 (RG-051)
pub fn create_number_format() -> Format {
    Format::new().set_num_format("#,##0.00")
}

/// Format entier #,##0
pub fn create_integer_format() -> Format {
    Format::new().set_num_format("#,##0")
}

/// Format pourcentage 0.0%
pub fn create_percent_format() -> Format {
    Format::new().set_num_format("0.0%")
}

/// Formatage conditionnel RAG 4 niveaux sur une colonne (RG-054).
/// `seuils` = (vert_max, jaune_max, orange_max)
/// Vert ≤ s1 | Jaune s1+1..=s2 | Orange s2+1..=s3 | Rouge > s3
pub fn apply_rag_conditional_format(
    ws: &mut Worksheet,
    first_row: u32,
    col: u16,
    last_row: u32,
    seuils: (u32, u32, u32),
) -> Result<(), XlsxError> {
    let (s1, s2, s3) = seuils;

    let green = Format::new()
        .set_background_color("C6EFCE")
        .set_font_color("006100");
    let yellow = Format::new()
        .set_background_color("FFEB9C")
        .set_font_color("9C6500");
    let orange = Format::new()
        .set_background_color("F4B084")
        .set_font_color("833C0C");
    let red = Format::new()
        .set_background_color("FFC7CE")
        .set_font_color("9C0006");

    ws.add_conditional_format(
        first_row,
        col,
        last_row,
        col,
        &ConditionalFormatCell::new()
            .set_rule(ConditionalFormatCellRule::LessThanOrEqualTo(s1 as f64))
            .set_format(&green),
    )?;
    ws.add_conditional_format(
        first_row,
        col,
        last_row,
        col,
        &ConditionalFormatCell::new()
            .set_rule(ConditionalFormatCellRule::Between(
                (s1 + 1) as f64,
                s2 as f64,
            ))
            .set_format(&yellow),
    )?;
    ws.add_conditional_format(
        first_row,
        col,
        last_row,
        col,
        &ConditionalFormatCell::new()
            .set_rule(ConditionalFormatCellRule::Between(
                (s2 + 1) as f64,
                s3 as f64,
            ))
            .set_format(&orange),
    )?;
    ws.add_conditional_format(
        first_row,
        col,
        last_row,
        col,
        &ConditionalFormatCell::new()
            .set_rule(ConditionalFormatCellRule::GreaterThan(s3 as f64))
            .set_format(&red),
    )?;

    Ok(())
}
