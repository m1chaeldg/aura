use crate::cst::*;

/// Losslessly serializes a CstDocument back to a String.
/// Invariant: `serialize(&parse(input).unwrap()) == input`
pub fn serialize(doc: &CstDocument) -> String {
    let mut out = String::new();

    for item in &doc.items {
        match item {
            CstTopLevelItem::Trivia(s) => out.push_str(s),
            CstTopLevelItem::KeyValue(kv) => out.push_str(&kv.raw),
            CstTopLevelItem::Table(table) => {
                out.push_str(&table.header_raw);
                for sub in &table.items {
                    match sub {
                        CstTableItem::Trivia(s) => out.push_str(s),
                        CstTableItem::KeyValue(kv) => out.push_str(&kv.raw),
                        CstTableItem::FencedBlock(fb) => out.push_str(&fb.raw),
                    }
                }
            }
            CstTopLevelItem::ConflictBlock(cb) => out.push_str(&cb.raw),
        }
    }

    out
}
