//! Lock ordering validator - ensures proper lock ordering to prevent deadlocks

use std::collections::HashMap;

/// Lock ordering configuration
#[derive(Debug, Clone)]
pub struct LockOrderingConfig {
    /// Ordered list of locks (must be acquired in this order)
    pub lock_order: Vec<String>,
    /// Lock acquisition timeout in milliseconds
    pub timeout_ms: u64,
}

/// Lock acquisition tracker
#[derive(Debug, Clone)]
pub struct LockTracker {
    pub thread_id: String,
    pub acquired_locks: Vec<String>,
    pub lock_times: HashMap<String, std::time::SystemTime>,
}

/// Thread-safe lock validator
pub struct LockValidator {
    config: LockOrderingConfig,
    trackers: std::sync::Arc<std::sync::Mutex<Vec<LockTracker>>>,
}

impl LockValidator {
    /// Create a new lock validator
    pub fn new(config: LockOrderingConfig) -> Self {
        Self {
            config,
            trackers: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    /// Register a lock acquisition attempt
    pub fn on_lock_acquire(&self, lock_name: &str) -> Result<(), String> {
        let thread_id = format!("{:?}", std::thread::current().id());

        // Find current thread's tracker
        let mut trackers = self.trackers.lock().map_err(|e| e.to_string())?;

        let tracker = trackers
            .iter_mut()
            .find(|t| t.thread_id == thread_id)
            .unwrap_or_else(|| {
                trackers.push(LockTracker {
                    thread_id: thread_id.clone(),
                    acquired_locks: Vec::new(),
                    lock_times: HashMap::new(),
                });
                trackers.last_mut().unwrap()
            });

        // Check lock ordering
        if let Some(last_lock) = tracker.acquired_locks.last() {
            let last_index = self.config.lock_order.iter().position(|l| l == last_lock);
            let current_index = self.config.lock_order.iter().position(|l| l == lock_name);

            match (last_index, current_index) {
                (Some(last_idx), Some(curr_idx)) => {
                    if curr_idx <= last_idx {
                        return Err(format!(
                            "Lock ordering violation: {} (index {}) before {} (index {})",
                            last_lock, last_idx, lock_name, curr_idx
                        ));
                    }
                }
                _ => {
                    return Err(format!(
                        "Unknown lock in order: {}",
                        if last_index.is_none() { last_lock } else { lock_name }
                    ));
                }
            }
        }

        // Record lock acquisition
        tracker.acquired_locks.push(lock_name.to_string());
        tracker.lock_times.insert(
            lock_name.to_string(),
            std::time::SystemTime::now(),
        );

        Ok(())
    }

    /// Register a lock release
    pub fn on_lock_release(&self, lock_name: &str) -> Result<(), String> {
        let thread_id = format!("{:?}", std::thread::current().id());

        let mut trackers = self.trackers.lock().map_err(|e| e.to_string())?;

        if let Some(tracker) = trackers.iter_mut().find(|t| t.thread_id == thread_id) {
            // Locks should be released in reverse order
            if let Some(last_lock) = tracker.acquired_locks.last() {
                if last_lock != lock_name {
                    return Err(format!(
                        "Lock release order violation: released {} but last acquired was {}",
                        lock_name, last_lock
                    ));
                }
            }

            tracker.acquired_locks.pop();
            tracker.lock_times.remove(lock_name);
        }

        Ok(())
    }

    /// Check if locks are held for too long
    pub fn check_lock_contention(&self) -> Result<Vec<String>, String> {
        let trackers = self.trackers.lock().map_err(|e| e.to_string())?;
        let mut warnings = Vec::new();

        for tracker in trackers.iter() {
            for (lock_name, acquire_time) in &tracker.lock_times {
                let elapsed = std::time::SystemTime::now()
                    .duration_since(*acquire_time)
                    .unwrap_or_default();

                if elapsed.as_millis() > self.config.timeout_ms as u128 {
                    warnings.push(format!(
                        "Thread {} held lock {} for {} ms (max: {})",
                        tracker.thread_id,
                        lock_name,
                        elapsed.as_millis(),
                        self.config.timeout_ms
                    ));
                }
            }
        }

        Ok(warnings)
    }

    /// Validate no circular lock dependencies
    pub fn validate_no_circular_dependencies(&self) -> Result<(), String> {
        // Check if lock order forms a DAG (directed acyclic graph)
        // For simplicity, verify that the order list has no duplicates
        let mut seen = std::collections::HashSet::new();

        for lock_name in &self.config.lock_order {
            if !seen.insert(lock_name) {
                return Err(format!("Circular dependency: {} appears multiple times", lock_name));
            }
        }

        Ok(())
    }

    /// Get lock order
    pub fn get_lock_order(&self) -> &[String] {
        &self.config.lock_order
    }

    /// Get tracker summary
    pub fn get_trackers_summary(&self) -> Result<Vec<(String, usize)>, String> {
        let trackers = self.trackers.lock().map_err(|e| e.to_string())?;

        Ok(trackers
            .iter()
            .map(|t| (t.thread_id.clone(), t.acquired_locks.len()))
            .collect())
    }
}

/// Thread-safe operations wrapper
pub struct ThreadSafeOperation {
    pub name: String,
    pub required_locks: Vec<String>,
}

impl ThreadSafeOperation {
    /// Create a new thread-safe operation spec
    pub fn new(name: &str, required_locks: Vec<String>) -> Self {
        Self {
            name: name.to_string(),
            required_locks,
        }
    }

    /// Validate locks are in correct order
    pub fn validate_lock_order(&self, lock_order: &[String]) -> Result<(), String> {
        let mut prev_index = -1i32;

        for lock in &self.required_locks {
            let current_index = lock_order
                .iter()
                .position(|l| l == lock)
                .ok_or_else(|| format!("Lock {} not found in global order", lock))? as i32;

            if current_index <= prev_index {
                return Err(format!(
                    "Lock order violation in {}: {} at index {} after index {}",
                    self.name, lock, current_index, prev_index
                ));
            }

            prev_index = current_index;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_creation() {
        let config = LockOrderingConfig {
            lock_order: vec![
                "signal_combiner".to_string(),
                "traders".to_string(),
            ],
            timeout_ms: 5000,
        };

        let validator = LockValidator::new(config);
        assert_eq!(validator.get_lock_order().len(), 2);
    }

    #[test]
    fn test_no_circular_dependencies() {
        let config = LockOrderingConfig {
            lock_order: vec![
                "signal_combiner".to_string(),
                "traders".to_string(),
                "positions".to_string(),
            ],
            timeout_ms: 5000,
        };

        let validator = LockValidator::new(config);
        assert!(validator.validate_no_circular_dependencies().is_ok());
    }

    #[test]
    fn test_lock_order_validation() {
        let op = ThreadSafeOperation::new(
            "execute_order",
            vec!["signal_combiner".to_string(), "traders".to_string()],
        );

        let lock_order = vec![
            "signal_combiner".to_string(),
            "traders".to_string(),
            "positions".to_string(),
        ];

        assert!(op.validate_lock_order(&lock_order).is_ok());
    }

    #[test]
    fn test_lock_order_violation() {
        let op = ThreadSafeOperation::new(
            "execute_order",
            vec!["traders".to_string(), "signal_combiner".to_string()],
        );

        let lock_order = vec![
            "signal_combiner".to_string(),
            "traders".to_string(),
            "positions".to_string(),
        ];

        assert!(op.validate_lock_order(&lock_order).is_err());
    }
}
