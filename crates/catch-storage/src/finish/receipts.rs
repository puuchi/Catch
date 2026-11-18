use crate::{Disk, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, sync::Arc};

pub(super) const FILE: &str = "catch-finish-receipts.json";
pub(super) const BACKUP: &str = "catch-finish-receipts.json.bak";
pub const MAX_RESPONSES: usize = 32;
pub const RESPONSE_BUDGET: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Receipt {
    pub request_id: String,
    fingerprint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<String>,
}
fn fingerprint(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut encoded = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut encoded, "{byte:02x}").unwrap();
    }
    encoded
}
impl Receipt {
    pub fn new(request_id: String, request: &[u8], response: String) -> Self {
        Self {
            request_id,
            fingerprint: fingerprint(request),
            response: Some(response),
        }
    }
    pub fn matches(&self, request: &[u8]) -> bool {
        self.fingerprint == fingerprint(request)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    version: u32,
    receipts: Vec<Receipt>,
}
#[derive(Deserialize)]
struct LegacyReceipt {
    request_id: String,
    fingerprint: Vec<u8>,
    response: Vec<u8>,
}
pub(super) fn read(disk: &dyn Disk) -> Result<Option<Vec<u8>>> {
    let bytes = disk.read(FILE)?;
    if bytes.is_none() && disk.read(BACKUP)?.is_some() {
        return Err("结束回执主文件缺失但备份存在，请先恢复文件".into());
    }
    Ok(bytes)
}
pub(super) fn decode(bytes: Option<&[u8]>) -> Result<(Vec<Receipt>, bool)> {
    let Some(bytes) = bytes else {
        return Ok((vec![], false));
    };
    let legacy = bytes.iter().find(|byte| !byte.is_ascii_whitespace()) == Some(&b'[');
    let rows = if legacy {
        let rows: Vec<LegacyReceipt> =
            serde_json::from_slice(bytes).map_err(|error| format!("结束请求回执无效：{error}"))?;
        rows.into_iter()
            .map(|row| {
                let response =
                    String::from_utf8(row.response).map_err(|_| "结束回执响应不是 UTF-8")?;
                Ok(Receipt::new(row.request_id, &row.fingerprint, response))
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        let document: Document =
            serde_json::from_slice(bytes).map_err(|error| format!("结束请求回执无效：{error}"))?;
        if document.version != 2 {
            return Err("无法识别结束回执版本，原件已保留".into());
        }
        document.receipts
    };
    let mut ids = std::collections::HashSet::new();
    if rows.iter().any(|row| {
        row.request_id.is_empty()
            || row.request_id.encode_utf16().count() > 100
            || !ids.insert(&row.request_id)
            || row.fingerprint.len() != 64
            || !row
                .fingerprint
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    }) {
        return Err("结束请求回执标识或参数摘要无效".into());
    }
    Ok((rows, legacy))
}
fn compact(rows: &mut [Receipt]) -> bool {
    let mut count = 0;
    let mut bytes = 0;
    let mut changed = false;
    for row in rows.iter_mut().rev() {
        if let Some(response) = &row.response {
            if count >= MAX_RESPONSES || response.len() > RESPONSE_BUDGET - bytes {
                row.response = None;
                changed = true;
            } else {
                count += 1;
                bytes += response.len();
            }
        }
    }
    changed
}
fn encode(rows: &[Receipt]) -> Result<Vec<u8>> {
    #[derive(Serialize)]
    struct BorrowedDocument<'a> {
        version: u32,
        receipts: &'a [Receipt],
    }
    serde_json::to_vec(&BorrowedDocument {
        version: 2,
        receipts: rows,
    })
    .map_err(|error| format!("结束回执编码失败：{error}"))
}

/// Request identities persist indefinitely. Only response bodies are bounded.
/// The desktop retains this index under the same mutex as the repositories.
pub struct ReceiptStore {
    pub(super) disk: Arc<dyn Disk>,
    pub(super) expected: Option<Vec<u8>>,
    rows: Vec<Receipt>,
    by_id: HashMap<String, usize>,
}
impl ReceiptStore {
    pub fn open(disk: Arc<dyn Disk>) -> Result<Self> {
        super::recover(disk.as_ref())?;
        let mut expected = read(disk.as_ref())?;
        let (mut rows, legacy) = decode(expected.as_deref())?;
        let compacted = compact(&mut rows);
        if legacy || compacted {
            let encoded = encode(&rows)?;
            if let Some(before) = &expected {
                disk.write_atomic(BACKUP, before)?;
            }
            disk.write_atomic(FILE, &encoded)?;
            expected = Some(encoded);
        }
        let mut store = Self {
            disk,
            expected: None,
            rows: vec![],
            by_id: HashMap::new(),
        };
        store.accept(rows, expected);
        Ok(store)
    }
    pub fn lookup(&self, request_id: &str) -> Option<&Receipt> {
        self.by_id.get(request_id).map(|index| &self.rows[*index])
    }
    pub(super) fn prepare(&self, receipt: Receipt) -> Result<(Vec<Receipt>, Vec<u8>)> {
        if self.lookup(&receipt.request_id).is_some() {
            return Err("结束请求已经提交，请读取回执".into());
        }
        let mut rows = self.rows.clone();
        rows.push(receipt);
        compact(&mut rows);
        let encoded = encode(&rows)?;
        // Keep the commit boundary strict even for callers outside Service.
        decode(Some(&encoded))?;
        Ok((rows, encoded))
    }
    pub(super) fn accept(&mut self, rows: Vec<Receipt>, expected: Option<Vec<u8>>) {
        self.by_id = rows
            .iter()
            .enumerate()
            .map(|(index, row)| (row.request_id.clone(), index))
            .collect();
        self.rows = rows;
        self.expected = expected;
    }
}
