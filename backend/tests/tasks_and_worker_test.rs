//! Integration tests for FileDeck's asynchronous task worker tracking and status lifecycle.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Started,
    Success,
    Failure,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskRecord {
    pub id: i64,
    pub task_id: String,
    pub task_name: String,
    pub status: TaskStatus,
    pub date_created: String,
    pub date_done: Option<String>,
    pub result: Option<String>,
    pub acknowledged: bool,
    pub related_document_id: Option<i64>,
}

#[test]
fn test_task_status_transitions() {
    let mut task = TaskRecord {
        id: 1,
        task_id: "task-uuid-99120-abc".to_string(),
        task_name: "consume_file: Invoice_2024.pdf".to_string(),
        status: TaskStatus::Pending,
        date_created: "2024-10-24T22:00:00Z".to_string(),
        date_done: None,
        result: None,
        acknowledged: false,
        related_document_id: None,
    };

    assert_eq!(task.status, TaskStatus::Pending);

    // Transition to Started
    task.status = TaskStatus::Started;
    assert_eq!(task.status, TaskStatus::Started);

    // Transition to Success
    task.status = TaskStatus::Success;
    task.date_done = Some("2024-10-24T22:00:04Z".to_string());
    task.result = Some("Document consumed successfully with ASN 1024".to_string());
    task.related_document_id = Some(42);

    assert_eq!(task.status, TaskStatus::Success);
    assert_eq!(task.related_document_id, Some(42));
    assert!(task.result.is_some());
}

#[test]
fn test_task_failure_payload() {
    let task = TaskRecord {
        id: 2,
        task_id: "task-uuid-failed-123".to_string(),
        task_name: "ocr_process: Corrupted.pdf".to_string(),
        status: TaskStatus::Failure,
        date_created: "2024-10-24T22:05:00Z".to_string(),
        date_done: Some("2024-10-24T22:05:01Z".to_string()),
        result: Some("Error: PDF syntax corrupted or encrypted with unsupported cipher".to_string()),
        acknowledged: false,
        related_document_id: None,
    };

    let json_val = serde_json::to_value(&task).unwrap();
    assert_eq!(json_val["status"], "Failure");
    assert!(json_val["result"].as_str().unwrap().contains("PDF syntax corrupted"));
}

#[test]
fn test_task_acknowledgment_filtering() {
    let tasks = vec![
        TaskRecord {
            id: 1,
            task_id: "t1".to_string(),
            task_name: "doc 1".to_string(),
            status: TaskStatus::Success,
            date_created: "2024-10-24T22:00:00Z".to_string(),
            date_done: Some("2024-10-24T22:00:02Z".to_string()),
            result: None,
            acknowledged: true,
            related_document_id: Some(10),
        },
        TaskRecord {
            id: 2,
            task_id: "t2".to_string(),
            task_name: "doc 2".to_string(),
            status: TaskStatus::Failure,
            date_created: "2024-10-24T22:01:00Z".to_string(),
            date_done: Some("2024-10-24T22:01:03Z".to_string()),
            result: Some("Network timeout".to_string()),
            acknowledged: false,
            related_document_id: None,
        },
    ];

    let unacknowledged: Vec<&TaskRecord> = tasks.iter().filter(|t| !t.acknowledged).collect();
    assert_eq!(unacknowledged.len(), 1);
    assert_eq!(unacknowledged[0].task_id, "t2");
}
