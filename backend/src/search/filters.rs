//! Document filter set and SQL translation for documents.
//!
//! Every query-string parameter that is not a pagination/search/ordering control is
//! interpreted as a filter and appended to the `WHERE` clause as an `AND` condition.

use crate::error::AppError;
use serde_json::Value;
use sqlx::{QueryBuilder, Sqlite, SqlitePool};
use std::collections::HashMap;

pub type Params = [(String, String)];

pub fn get<'a>(params: &'a Params, key: &str) -> Option<&'a str> {
    params
        .iter()
        .rev()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

fn truthy(v: &str) -> bool {
    matches!(v.to_ascii_lowercase().as_str(), "true" | "1" | "yes" | "on")
}

fn ids(v: &str) -> Vec<i64> {
    v.split(',').filter_map(|s| s.trim().parse().ok()).collect()
}

fn id_list(v: &[i64]) -> String {
    if v.is_empty() {
        "(NULL)".to_string()
    } else {
        format!(
            "({})",
            v.iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(",")
        )
    }
}

fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// Text comparison on a SQL expression. Returns false if `op` is not a text op.
fn text_op(qb: &mut QueryBuilder<'_, Sqlite>, col: &str, op: &str, val: &str) -> bool {
    let (pre, post) = match op {
        "icontains" => ("%", "%"),
        "istartswith" => ("", "%"),
        "iendswith" => ("%", ""),
        "iexact" => {
            qb.push(format!(" AND LOWER({col}) = LOWER("));
            qb.push_bind(val.to_string());
            qb.push(")");
            return true;
        }
        _ => return false,
    };
    qb.push(format!(" AND {col} LIKE "));
    qb.push_bind(format!("{pre}{}{post}", escape_like(val)));
    qb.push(" ESCAPE '\\'");
    true
}

fn cmp_sym(op: &str) -> Option<&'static str> {
    Some(match op {
        "exact" => "=",
        "gt" => ">",
        "gte" => ">=",
        "lt" => "<",
        "lte" => "<=",
        _ => return None,
    })
}

fn int_op(qb: &mut QueryBuilder<'_, Sqlite>, col: &str, op: &str, val: &str) -> bool {
    if op == "isnull" {
        qb.push(format!(
            " AND {col} IS {}NULL",
            if truthy(val) { "" } else { "NOT " }
        ));
        return true;
    }
    match (cmp_sym(op), val.trim().parse::<i64>()) {
        (Some(sym), Ok(n)) => {
            qb.push(format!(" AND {col} {sym} "));
            qb.push_bind(n);
            true
        }
        _ => false,
    }
}

/// Date / datetime columns stored as ISO-8601 text.
fn date_op(qb: &mut QueryBuilder<'_, Sqlite>, col: &str, op: &str, val: &str) -> bool {
    let (expr, op) = match op.strip_prefix("date__") {
        Some(rest) => (format!("date({col})"), rest),
        None => (
            format!("substr({col},1,{})", if val.len() <= 10 { 10 } else { 19 }),
            op,
        ),
    };
    match op {
        "year" | "month" | "day" => {
            let fmt = match op {
                "year" => "%Y",
                "month" => "%m",
                _ => "%d",
            };
            if let Ok(n) = val.trim().parse::<i64>() {
                qb.push(format!(" AND CAST(strftime('{fmt}', {col}) AS INTEGER) = "));
                qb.push_bind(n);
                return true;
            }
            false
        }
        "gt" | "gte" | "lt" | "lte" => {
            let sym = cmp_sym(op).unwrap();
            qb.push(format!(" AND {expr} {sym} "));
            qb.push_bind(val.to_string());
            true
        }
        _ => false,
    }
}

fn exists_tag(neg: bool, cond: &str) -> String {
    format!(
        " AND {}EXISTS (SELECT 1 FROM document_tags dt WHERE dt.document_id = d.id{cond})",
        if neg { "NOT " } else { "" }
    )
}

fn exists_cf(neg: bool, cond: &str) -> String {
    format!(
        " AND {}EXISTS (SELECT 1 FROM custom_field_instances i WHERE i.document_id = d.id AND i.deleted_at IS NULL{cond})",
        if neg { "NOT " } else { "" }
    )
}

struct Rel {
    name: &'static str,
    col: &'static str,
    table: &'static str,
}

const RELS: [Rel; 3] = [
    Rel {
        name: "correspondent",
        col: "d.correspondent_id",
        table: "correspondents",
    },
    Rel {
        name: "document_type",
        col: "d.document_type_id",
        table: "document_types",
    },
    Rel {
        name: "storage_path",
        col: "d.storage_path_id",
        table: "storage_paths",
    },
];

/// Control parameters that are not filters.
const CONTROL: &[&str] = &[
    "page",
    "page_size",
    "ordering",
    "text",
    "title_search",
    "query",
    "more_like_id",
    "truncate_content",
    "fields",
    "full_perms",
    "include_selection_data",
    "format",
    "version",
];

pub async fn apply(
    qb: &mut QueryBuilder<'_, Sqlite>,
    db: &SqlitePool,
    params: &Params,
) -> Result<(), AppError> {
    for (k, v) in params {
        let k = k.as_str();
        let v = v.as_str();
        if CONTROL.contains(&k) || v.is_empty() {
            continue;
        }
        apply_one(qb, db, params, k, v).await?;
    }
    Ok(())
}

async fn apply_one(
    qb: &mut QueryBuilder<'_, Sqlite>,
    db: &SqlitePool,
    _params: &Params,
    k: &str,
    v: &str,
) -> Result<(), AppError> {
    let bad = || AppError::Validation(format!("Invalid value for filter '{k}'"));

    // ---- simple named filters ----
    match k {
        "is_tagged" => {
            qb.push(exists_tag(!truthy(v), ""));
            return Ok(());
        }
        "is_in_inbox" => {
            qb.push(exists_tag(
                !truthy(v),
                " AND dt.tag_id IN (SELECT id FROM tags WHERE is_inbox_tag = 1)",
            ));
            return Ok(());
        }
        "has_custom_fields" => {
            qb.push(exists_cf(!truthy(v), ""));
            return Ok(());
        }
        "has_duplicates" => {
            qb.push(format!(
                " AND {}EXISTS (SELECT 1 FROM documents o WHERE o.id != d.id AND o.root_document_id IS NULL \
                 AND o.deleted_at IS NULL AND (o.checksum = d.checksum \
                 OR (d.archive_checksum IS NOT NULL AND d.archive_checksum != '' AND (o.checksum = d.archive_checksum OR o.archive_checksum = d.archive_checksum)) \
                 OR (o.archive_checksum IS NOT NULL AND o.archive_checksum != '' AND o.archive_checksum = d.checksum)))",
                if truthy(v) { "" } else { "NOT " }
            ));
            return Ok(());
        }
        "mime_type" => {
            qb.push(" AND d.mime_type = ");
            qb.push_bind(v.to_string());
            return Ok(());
        }
        "title_content" => {
            let p = format!("%{}%", escape_like(v));
            qb.push(" AND (d.title LIKE ");
            qb.push_bind(p.clone());
            qb.push(" ESCAPE '\\' OR d.content LIKE ");
            qb.push_bind(p);
            qb.push(" ESCAPE '\\')");
            return Ok(());
        }
        "search" => {
            let p = format!("%{}%", escape_like(v));
            qb.push(" AND (d.title LIKE ");
            qb.push_bind(p.clone());
            qb.push(" ESCAPE '\\' OR d.content LIKE ");
            qb.push_bind(p.clone());
            qb.push(" ESCAPE '\\' OR d.correspondent_id IN (SELECT id FROM correspondents WHERE name LIKE ");
            qb.push_bind(p);
            qb.push(" ESCAPE '\\'))");
            return Ok(());
        }
        "custom_fields__icontains" => {
            let p = format!("%{}%", escape_like(v));
            qb.push(" AND EXISTS (SELECT 1 FROM custom_field_instances i JOIN custom_fields cf ON cf.id = i.field_id \
                      WHERE i.document_id = d.id AND i.deleted_at IS NULL AND (cf.name LIKE ");
            qb.push_bind(p.clone());
            for col in [
                "value_text",
                "value_url",
                "value_long_text",
                "value_select",
                "value_date",
                "value_monetary",
                "value_document_ids",
            ] {
                qb.push(format!(" ESCAPE '\\' OR i.{col} LIKE "));
                qb.push_bind(p.clone());
            }
            qb.push(" ESCAPE '\\'))");
            return Ok(());
        }
        "custom_field_query" => {
            let json: Value = serde_json::from_str(v).map_err(|_| bad())?;
            let rows = sqlx::query_as::<_, (i64, String, String)>(
                "SELECT id, name, data_type FROM custom_fields",
            )
            .fetch_all(db)
            .await?;
            let fields: HashMap<String, (i64, String)> =
                rows.into_iter().map(|(id, n, t)| (n, (id, t))).collect();
            qb.push(" AND ");
            let mut atoms = 0;
            cf_expr(qb, &fields, &json, 0, &mut atoms).map_err(AppError::Validation)?;
            return Ok(());
        }
        "custom_fields__id__all" | "custom_fields__id__none" | "custom_fields__id__in" => {
            let list = ids(v);
            match k {
                "custom_fields__id__all" => {
                    for id in list {
                        qb.push(exists_cf(false, &format!(" AND i.field_id = {id}")));
                    }
                }
                "custom_fields__id__none" => {
                    for id in list {
                        qb.push(exists_cf(true, &format!(" AND i.field_id = {id}")));
                    }
                }
                _ => {
                    qb.push(exists_cf(
                        false,
                        &format!(" AND i.field_id IN {}", id_list(&list)),
                    ));
                }
            }
            return Ok(());
        }
        // Deprecated aliases
        "created__date__gt" | "created__date__gte" | "created__date__lt" | "created__date__lte" => {
            let op = &k["created__date__".len()..];
            return if date_op(qb, "d.created", &format!("date__{op}"), v) {
                Ok(())
            } else {
                Err(bad())
            };
        }
        _ => {}
    }

    // ---- tags ----
    if let Some(rest) = k.strip_prefix("tags__") {
        match rest {
            "id" | "id__exact" => {
                let id: i64 = v.parse().map_err(|_| bad())?;
                qb.push(exists_tag(false, &format!(" AND dt.tag_id = {id}")));
            }
            "id__in" => {
                qb.push(exists_tag(
                    false,
                    &format!(" AND dt.tag_id IN {}", id_list(&ids(v))),
                ));
            }
            "id__all" => {
                for id in ids(v) {
                    qb.push(exists_tag(false, &format!(" AND dt.tag_id = {id}")));
                }
            }
            "id__none" => {
                for id in ids(v) {
                    qb.push(exists_tag(true, &format!(" AND dt.tag_id = {id}")));
                }
            }
            _ => {
                if let Some(op) = rest.strip_prefix("name__") {
                    qb.push(" AND EXISTS (SELECT 1 FROM document_tags dt JOIN tags tg ON tg.id = dt.tag_id WHERE dt.document_id = d.id");
                    if !text_op(qb, "tg.name", op, v) {
                        return Err(bad());
                    }
                    qb.push(")");
                } else {
                    return Err(bad());
                }
            }
        }
        return Ok(());
    }

    // ---- correspondent / document_type / storage_path ----
    for rel in &RELS {
        let Some(rest) = k.strip_prefix(rel.name) else {
            continue;
        };
        let rest = match rest.strip_prefix("__") {
            Some(r) => r,
            None => continue,
        };
        match rest {
            "isnull" => {
                qb.push(format!(
                    " AND {} IS {}NULL",
                    rel.col,
                    if truthy(v) { "" } else { "NOT " }
                ));
            }
            "id" | "id__exact" => {
                let id: i64 = v.parse().map_err(|_| bad())?;
                qb.push(format!(" AND {} = {id}", rel.col));
            }
            "id__in" => {
                qb.push(format!(" AND {} IN {}", rel.col, id_list(&ids(v))));
            }
            "id__none" => {
                qb.push(format!(
                    " AND ({c} IS NULL OR {c} NOT IN {l})",
                    c = rel.col,
                    l = id_list(&ids(v))
                ));
            }
            _ => {
                if let Some(op) = rest.strip_prefix("name__") {
                    qb.push(format!(
                        " AND {} IN (SELECT id FROM {} WHERE 1=1",
                        rel.col, rel.table
                    ));
                    if !text_op(qb, "name", op, v) {
                        return Err(bad());
                    }
                    qb.push(")");
                } else {
                    return Err(bad());
                }
            }
        }
        return Ok(());
    }

    // ---- scalar document columns ----
    let (field, op) = match k.split_once("__") {
        Some((f, o)) => (f, o),
        None => (k, "exact"),
    };
    let ok = match field {
        "id" => match op {
            "in" => {
                qb.push(format!(" AND d.id IN {}", id_list(&ids(v))));
                true
            }
            _ => int_op(qb, "d.id", op, v),
        },
        "title" => text_op(qb, "d.title", op, v),
        "original_filename" => text_op(qb, "d.original_filename", op, v),
        "checksum" => text_op(qb, "d.checksum", op, v),
        "content" => text_op(qb, "d.content", op, v),
        "archive_serial_number" => int_op(qb, "d.archive_serial_number", op, v),
        "created" => date_op(qb, "d.created", op, v),
        "added" => date_op(qb, "d.added", op, v),
        "modified" => date_op(qb, "d.modified", op, v),
        _ => false,
    };
    if ok {
        Ok(())
    } else {
        // Unknown filters are rejected, like django-filter does with strict mode.
        Err(AppError::Validation(format!(
            "Unknown or invalid filter '{k}'"
        )))
    }
}

// ---------------------------------------------------------------------------
// custom_field_query: ["AND", [expr, ...]] | ["OR", [...]] | ["NOT", expr] | [field, op, value]
// ---------------------------------------------------------------------------

const MAX_DEPTH: usize = 10;
const MAX_ATOMS: usize = 20;

fn value_column(data_type: &str) -> &'static str {
    match data_type {
        "string" => "i.value_text",
        "longtext" => "i.value_long_text",
        "url" => "i.value_url",
        "date" => "i.value_date",
        "boolean" => "i.value_bool",
        "integer" => "i.value_int",
        "float" => "i.value_float",
        "monetary" => "CAST(substr(i.value_monetary, 4) AS REAL)",
        "documentlink" => "i.value_document_ids",
        "select" => "i.value_select",
        _ => "i.value_text",
    }
}

fn push_json_bind(qb: &mut QueryBuilder<'_, Sqlite>, v: &Value) -> Result<(), String> {
    match v {
        Value::String(s) => {
            qb.push_bind(s.clone());
        }
        Value::Bool(b) => {
            qb.push_bind(*b as i64);
        }
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                qb.push_bind(i);
            } else if let Some(f) = n.as_f64() {
                qb.push_bind(f);
            }
        }
        _ => return Err("Unsupported value type in custom_field_query".into()),
    }
    Ok(())
}

fn cf_expr(
    qb: &mut QueryBuilder<'_, Sqlite>,
    fields: &HashMap<String, (i64, String)>,
    expr: &Value,
    depth: usize,
    atoms: &mut usize,
) -> Result<(), String> {
    if depth > MAX_DEPTH {
        return Err("custom_field_query is nested too deeply".into());
    }
    let arr = expr
        .as_array()
        .ok_or("custom_field_query expression must be a list")?;
    match arr.as_slice() {
        [Value::String(op), Value::Array(children)]
            if op.eq_ignore_ascii_case("AND") || op.eq_ignore_ascii_case("OR") =>
        {
            if children.is_empty() {
                qb.push("1=1");
                return Ok(());
            }
            let joiner = if op.eq_ignore_ascii_case("AND") {
                " AND "
            } else {
                " OR "
            };
            qb.push("(");
            for (n, c) in children.iter().enumerate() {
                if n > 0 {
                    qb.push(joiner);
                }
                cf_expr(qb, fields, c, depth + 1, atoms)?;
            }
            qb.push(")");
            Ok(())
        }
        [Value::String(op), child] if op.eq_ignore_ascii_case("NOT") => {
            qb.push("NOT (");
            cf_expr(qb, fields, child, depth + 1, atoms)?;
            qb.push(")");
            Ok(())
        }
        [Value::String(name), Value::String(op), value] => {
            *atoms += 1;
            if *atoms > MAX_ATOMS {
                return Err("custom_field_query has too many conditions".into());
            }
            let (fid, dtype) = fields
                .get(name)
                .ok_or_else(|| format!("Custom field '{name}' does not exist"))?;
            let col = value_column(dtype);
            let base = format!(
                "EXISTS (SELECT 1 FROM custom_field_instances i WHERE i.document_id = d.id AND i.deleted_at IS NULL AND i.field_id = {fid}"
            );
            match op.as_str() {
                "exists" => {
                    let want = value.as_bool().unwrap_or(true);
                    qb.push(format!("{}{base})", if want { "" } else { "NOT " }));
                }
                "isnull" => {
                    let want = value.as_bool().unwrap_or(true);
                    qb.push(format!(
                        "{}{base} AND {col} IS NOT NULL)",
                        if want { "NOT " } else { "" }
                    ));
                }
                "exact" | "gt" | "gte" | "lt" | "lte" => {
                    qb.push(format!("{base} AND {col} {} ", cmp_sym(op).unwrap()));
                    push_json_bind(qb, value)?;
                    qb.push(")");
                }
                "in" => {
                    let list = value.as_array().ok_or("'in' expects a list")?;
                    qb.push(format!("{base} AND {col} IN ("));
                    for (n, item) in list.iter().enumerate() {
                        if n > 0 {
                            qb.push(", ");
                        }
                        push_json_bind(qb, item)?;
                    }
                    qb.push("))");
                }
                "range" => {
                    let r = value
                        .as_array()
                        .filter(|a| a.len() == 2)
                        .ok_or("'range' expects [low, high]")?;
                    qb.push(format!("{base} AND {col} BETWEEN "));
                    push_json_bind(qb, &r[0])?;
                    qb.push(" AND ");
                    push_json_bind(qb, &r[1])?;
                    qb.push(")");
                }
                "contains" => {
                    // document links: JSON list of ids; value is a list of ids that must all be present
                    let list: Vec<i64> = match value {
                        Value::Array(a) => a.iter().filter_map(|x| x.as_i64()).collect(),
                        other => other.as_i64().into_iter().collect(),
                    };
                    qb.push("(");
                    for (n, id) in list.iter().enumerate() {
                        if n > 0 {
                            qb.push(" AND ");
                        }
                        qb.push(format!(
                            "{base} AND (i.value_document_ids LIKE '%[{id}]%' OR i.value_document_ids LIKE '%[{id},%' \
                             OR i.value_document_ids LIKE '%,{id},%' OR i.value_document_ids LIKE '%,{id}]%'))"
                        ));
                    }
                    if list.is_empty() {
                        qb.push("1=1");
                    }
                    qb.push(")");
                }
                "icontains" | "istartswith" | "iendswith" => {
                    let s = value.as_str().ok_or("string operator expects a string")?;
                    let (pre, post) = match op.as_str() {
                        "icontains" => ("%", "%"),
                        "istartswith" => ("", "%"),
                        _ => ("%", ""),
                    };
                    qb.push(format!("{base} AND {col} LIKE "));
                    qb.push_bind(format!("{pre}{}{post}", escape_like(s)));
                    qb.push(" ESCAPE '\\')");
                }
                other => return Err(format!("Invalid custom_field_query operator '{other}'")),
            }
            Ok(())
        }
        _ => Err("Invalid custom_field_query expression".into()),
    }
}

// ---------------------------------------------------------------------------
// Ordering
// ---------------------------------------------------------------------------

/// Translate a DRF-style `ordering` value into a SQL `ORDER BY` fragment (without the keyword).
/// Returns None for unknown fields and for `score`, which is handled by the caller.
pub fn order_by(ordering: &str) -> Option<String> {
    let mut parts = Vec::new();
    for raw in ordering.split(',') {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        let (desc, name) = match raw.strip_prefix('-') {
            Some(n) => (true, n),
            None => (false, raw),
        };
        let expr = match name {
            "id" => "d.id".to_string(),
            "title" => "LOWER(d.title)".to_string(),
            "correspondent__name" => {
                "(SELECT LOWER(name) FROM correspondents WHERE id = d.correspondent_id)".to_string()
            }
            "document_type__name" => {
                "(SELECT LOWER(name) FROM document_types WHERE id = d.document_type_id)".to_string()
            }
            "storage_path__name" => {
                "(SELECT LOWER(name) FROM storage_paths WHERE id = d.storage_path_id)".to_string()
            }
            "created" => "d.created".to_string(),
            "modified" => "d.modified".to_string(),
            "added" => "d.added".to_string(),
            "archive_serial_number" => "d.archive_serial_number".to_string(),
            "page_count" => "d.page_count".to_string(),
            "num_notes" => {
                "(SELECT COUNT(*) FROM notes n WHERE n.document_id = d.id AND n.deleted_at IS NULL)"
                    .to_string()
            }
            "owner" => "d.id".to_string(),
            n if n.starts_with("custom_field_") => {
                let id: i64 = n["custom_field_".len()..].parse().ok()?;
                format!(
                    "(SELECT COALESCE(i.value_text, i.value_int, i.value_float, i.value_date, i.value_select, i.value_url, i.value_long_text, i.value_bool) \
                     FROM custom_field_instances i WHERE i.document_id = d.id AND i.field_id = {id} AND i.deleted_at IS NULL LIMIT 1)"
                )
            }
            _ => return None,
        };
        // NULLs last regardless of direction
        parts.push(format!(
            "({expr}) IS NULL, {expr} {}",
            if desc { "DESC" } else { "ASC" }
        ));
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(", "))
    }
}
