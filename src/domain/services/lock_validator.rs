//! Lock ordering validator - ensures proper lock ordering to prevent deadlocks
//! Advanced concurrency safety with deadlock detection, RwLock semantics, and performance monitoring

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// Lock ordering configuration
#[derive(Debug, Clone)]
pub struct LockOrderingConfig {
    /// Ordered list of locks (must be acquired in this order)
    pub lock_order: Vec<String>,
    /// Lock acquisition timeout in milliseconds
    pub timeout_ms: u64,
    /// Maximum lock hold time in milliseconds
    pub max_hold_ms: u64,
    /// Enable deadlock detection
    pub deadlock_detection_enabled: bool,
}

/// Lock acquisition tracker with enhanced deadlock detection
#[derive(Debug, Clone)]
pub struct LockTracker {
    pub thread_id: String,
    pub acquired_locks: Vec<String>,
    pub lock_times: HashMap<String, SystemTime>,
    pub waiting_for: Option<String>,
    pub wait_start_time: Option<SystemTime>,
    pub is_writer: HashMap<String, bool>, // true for write locks, false for read locks
}

/// Thread waiting information for deadlock detection
#[derive(Debug, Clone)]
pub struct ThreadWaitInfo {
    pub thread_id: String,
    pub waiting_for_lock: String,
    pub wait_duration_ms: u64,
    pub acquired_locks: Vec<String>,
}

/// Lock statistics for monitoring
#[derive(Debug, Clone)]
pub struct LockStats {
    pub total_acquisitions: u64,
    pub total_holds_ms: u64,
    pub max_hold_ms: u64,
    pub contention_count: u64,
    pub last_acquired: Option<SystemTime>,
}

/// Lock dependency graph for deadlock detection
#[derive(Debug, Clone)]
pub struct LockGraph {
    pub dependencies: HashMap<String, HashSet<String>>, // lock -> threads waiting for it
    pub thread_holds: HashMap<String, HashSet<String>>, // thread -> locks it holds
}

/// Critical section analysis
#[derive(Debug, Clone)]
pub struct CriticalSectionAnalysis {
    pub section_name: String,
    pub lock_hold_times: HashMap<String, u64>,
    pub total_duration_ms: u64,
    pub violations: Vec<String>,
}

/// Thread-safe lock validator with advanced features
pub struct LockValidator {
    config: LockOrderingConfig,
    trackers: Arc<tokio::sync::Mutex<Vec<LockTracker>>>,
    lock_stats: Arc<tokio::sync::Mutex<HashMap<String, LockStats>>>,
    waiting_queue: Arc<tokio::sync::Mutex<HashMap<String, VecDeque<String>>>>, // lock -> waiting threads FIFO
    rwlock_readers: Arc<tokio::sync::Mutex<HashMap<String, HashSet<String>>>>, // lock -> active readers
    rwlock_writers: Arc<tokio::sync::Mutex<HashMap<String, HashSet<String>>>>, // lock -> active writers
}

impl LockValidator {
    /// Create a new lock validator
    pub fn new(config: LockOrderingConfig) -> Self {
        Self {
            trackers: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            lock_stats: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            waiting_queue: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            rwlock_readers: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            rwlock_writers: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            config,
        }
    }

    /// Validate lock order for a sequence of locks
    pub fn validate_lock_order(&self, locks: &[&str]) -> Result<(), String> {
        let mut prev_index = -1i32;

        for lock in locks {
            let current_index = self.config.lock_order
                .iter()
                .position(|l| l == lock)
                .ok_or_else(|| format!("Lock {} not found in global order", lock))? as i32;

            if current_index <= prev_index {
                return Err(format!(
                    "Lock ordering violation: {} at index {} after index {}",
                    lock, current_index, prev_index
                ));
            }

            prev_index = current_index;
        }

        Ok(())
    }

    /// Detect potential deadlocks using cycle detection
    pub async fn detect_deadlock(&self) -> Result<(), String> {
        if !self.config.deadlock_detection_enabled {
            return Ok(());
        }

        let trackers = self.trackers.lock().await;
        let mut graph = LockGraph {
            dependencies: HashMap::new(),
            thread_holds: HashMap::new(),
        };

        // Build dependency graph
        for tracker in trackers.iter() {
            if let Some(waiting_lock) = &tracker.waiting_for {
                graph.dependencies
                    .entry(waiting_lock.clone())
                    .or_insert_with(HashSet::new)
                    .insert(tracker.thread_id.clone());
            }

            graph.thread_holds
                .entry(tracker.thread_id.clone())
                .or_insert_with(HashSet::new)
                .extend(tracker.acquired_locks.iter().cloned());
        }

        // Detect cycles using DFS (simplified version without recursion)
        let mut visited = HashSet::new();
        let mut recursion_stack = HashSet::new();

        for thread_id in graph.thread_holds.keys() {
            if self.has_cycle(thread_id, &graph, &mut visited, &mut recursion_stack).await {
                return Err(format!("Deadlock detected involving thread {}", thread_id));
            }
        }

        Ok(())
    }

    /// Helper for cycle detection in dependency graph (simplified non-recursive version)
    async fn has_cycle(&self, start_thread: &str, graph: &LockGraph, visited: &mut HashSet<String>, recursion_stack: &mut HashSet<String>) -> bool {
        // For simplicity, just check if any thread is waiting for a lock held by another thread
        // that is also waiting (basic cycle detection)
        for tracker in graph.thread_holds.keys() {
            if let Some(waiting_lock) = self.get_waiting_lock(tracker).await {
                for (other_thread, held_locks) in &graph.thread_holds {
                    if other_thread != tracker && held_locks.contains(&waiting_lock) {
                        if let Some(other_waiting) = self.get_waiting_lock(other_thread).await {
                            // Check if other_thread is waiting for a lock held by the original thread
                            if graph.thread_holds.get(tracker).map_or(false, |locks| locks.contains(&other_waiting)) {
                                return true; // Found a cycle
                            }
                        }
                    }
                }
            }
        }
        false
    }

    /// Get the lock a thread is currently waiting for
    async fn get_waiting_lock(&self, thread_id: &str) -> Option<String> {
        let trackers = self.trackers.lock().await;
        trackers.iter()
            .find(|t| t.thread_id == thread_id)?
            .waiting_for
            .clone()
    }

    /// Validate RwLock semantics
    pub async fn validate_rwlock_semantics(&self, lock: &str, is_write: bool) -> Result<(), String> {
        let readers = self.rwlock_readers.lock().await;
        let writers = self.rwlock_writers.lock().await;

        if is_write {
            // Write lock: no other readers or writers allowed
            if readers.get(lock).map_or(false, |r| !r.is_empty()) {
                return Err(format!("Write lock on {} blocked by active readers", lock));
            }
            if writers.get(lock).map_or(false, |w| !w.is_empty()) {
                return Err(format!("Write lock on {} blocked by active writer", lock));
            }
        } else {
            // Read lock: no writers allowed
            if writers.get(lock).map_or(false, |w| !w.is_empty()) {
                return Err(format!("Read lock on {} blocked by active writer", lock));
            }
        }

        Ok(())
    }

    /// Record lock acquisition
    pub async fn record_lock_acquisition(&mut self, lock: &str, thread_id: &str, is_write: bool) -> Result<(), String> {
        let mut trackers = self.trackers.lock().await;
        let mut stats = self.lock_stats.lock().await;
        let mut readers = self.rwlock_readers.lock().await;
        let mut writers = self.rwlock_writers.lock().await;

        // Find or create tracker
        if !trackers.iter().any(|t| t.thread_id == thread_id) {
            trackers.push(LockTracker {
                thread_id: thread_id.to_string(),
                acquired_locks: Vec::new(),
                lock_times: HashMap::new(),
                waiting_for: None,
                wait_start_time: None,
                is_writer: HashMap::new(),
            });
        }

        let tracker = trackers.iter_mut().find(|t| t.thread_id == thread_id).unwrap();

        // Clear waiting state
        tracker.waiting_for = None;
        tracker.wait_start_time = None;

        // Record acquisition
        tracker.acquired_locks.push(lock.to_string());
        tracker.lock_times.insert(lock.to_string(), SystemTime::now());
        tracker.is_writer.insert(lock.to_string(), is_write);

        // Update RwLock tracking
        if is_write {
            writers.entry(lock.to_string()).or_insert_with(HashSet::new).insert(thread_id.to_string());
        } else {
            readers.entry(lock.to_string()).or_insert_with(HashSet::new).insert(thread_id.to_string());
        }

        // Update stats
        let stat = stats.entry(lock.to_string()).or_insert(LockStats {
            total_acquisitions: 0,
            total_holds_ms: 0,
            max_hold_ms: 0,
            contention_count: 0,
            last_acquired: None,
        });
        stat.total_acquisitions += 1;
        stat.last_acquired = Some(SystemTime::now());

        Ok(())
    }

    /// Record lock release
    pub async fn record_lock_release(&mut self, lock: &str, thread_id: &str) -> Result<(), String> {
        let mut trackers = self.trackers.lock().await;
        let mut stats = self.lock_stats.lock().await;
        let mut readers = self.rwlock_readers.lock().await;
        let mut writers = self.rwlock_writers.lock().await;

        let tracker = trackers.iter_mut().find(|t| t.thread_id == thread_id)
            .ok_or_else(|| format!("No tracker found for thread {}", thread_id))?;

        // Verify LIFO release order
        if let Some(last_lock) = tracker.acquired_locks.last() {
            if last_lock != lock {
                return Err(format!(
                    "Lock release order violation: released {} but last acquired was {}",
                    lock, last_lock
                ));
            }
        }

        // Calculate hold time
        if let Some(acquire_time) = tracker.lock_times.get(lock) {
            let hold_duration = SystemTime::now().duration_since(*acquire_time)
                .unwrap_or_default();
            let hold_ms = hold_duration.as_millis() as u64;

            // Update stats
            if let Some(stat) = stats.get_mut(lock) {
                stat.total_holds_ms += hold_ms;
                stat.max_hold_ms = stat.max_hold_ms.max(hold_ms);
            }

            // Check for excessive hold time
            if hold_ms > self.config.max_hold_ms {
                return Err(format!(
                    "Lock {} held for {}ms (max allowed: {}ms)",
                    lock, hold_ms, self.config.max_hold_ms
                ));
            }
        }

        // Remove from acquired locks
        tracker.acquired_locks.pop();
        tracker.lock_times.remove(lock);

        // Update RwLock tracking
        if let Some(is_write) = tracker.is_writer.remove(lock) {
            if is_write {
                if let Some(writer_set) = writers.get_mut(lock) {
                    writer_set.remove(thread_id);
                }
            } else {
                if let Some(reader_set) = readers.get_mut(lock) {
                    reader_set.remove(thread_id);
                }
            }
        }

        Ok(())
    }

    /// Get lock contention statistics
    pub async fn get_lock_contention_stats(&self) -> HashMap<String, LockStats> {
        self.lock_stats.lock().await.clone()
    }

    /// Get information about waiting threads
    pub async fn get_waiting_threads(&self) -> Vec<ThreadWaitInfo> {
        let trackers = self.trackers.lock().await;
        let mut result = Vec::new();

        for tracker in trackers.iter() {
            if let (Some(waiting_lock), Some(wait_start)) = (&tracker.waiting_for, &tracker.wait_start_time) {
                let wait_duration = SystemTime::now().duration_since(*wait_start)
                    .unwrap_or_default();

                result.push(ThreadWaitInfo {
                    thread_id: tracker.thread_id.clone(),
                    waiting_for_lock: waiting_lock.clone(),
                    wait_duration_ms: wait_duration.as_millis() as u64,
                    acquired_locks: tracker.acquired_locks.clone(),
                });
            }
        }

        result
    }

    /// Get current lock dependency graph
    pub async fn get_lock_graph(&self) -> LockGraph {
        let trackers = self.trackers.lock().await;
        let mut graph = LockGraph {
            dependencies: HashMap::new(),
            thread_holds: HashMap::new(),
        };

        for tracker in trackers.iter() {
            if let Some(waiting_lock) = &tracker.waiting_for {
                graph.dependencies
                    .entry(waiting_lock.clone())
                    .or_insert_with(HashSet::new)
                    .insert(tracker.thread_id.clone());
            }

            graph.thread_holds
                .entry(tracker.thread_id.clone())
                .or_insert_with(HashSet::new)
                .extend(tracker.acquired_locks.iter().cloned());
        }

        graph
    }

    /// Analyze critical section timing
    pub async fn validate_critical_section(&self, section: &str) -> CriticalSectionAnalysis {
        let trackers = self.trackers.lock().await;
        let mut analysis = CriticalSectionAnalysis {
            section_name: section.to_string(),
            lock_hold_times: HashMap::new(),
            total_duration_ms: 0,
            violations: Vec::new(),
        };

        for tracker in trackers.iter() {
            for (lock_name, acquire_time) in &tracker.lock_times {
                let hold_duration = SystemTime::now().duration_since(*acquire_time)
                    .unwrap_or_default();
                let hold_ms = hold_duration.as_millis() as u64;

                analysis.lock_hold_times.insert(
                    format!("{}:{}", tracker.thread_id, lock_name),
                    hold_ms
                );

                if hold_ms > self.config.max_hold_ms {
                    analysis.violations.push(format!(
                        "Thread {} held lock {} for {}ms (max: {}ms)",
                        tracker.thread_id, lock_name, hold_ms, self.config.max_hold_ms
                    ));
                }
            }
        }

        analysis
    }

    /// Register that a thread is waiting for a lock
    pub async fn record_thread_waiting(&mut self, lock: &str, thread_id: &str) -> Result<(), String> {
        let mut trackers = self.trackers.lock().await;
        let mut queue = self.waiting_queue.lock().await;

        // Find or create tracker
        if !trackers.iter().any(|t| t.thread_id == thread_id) {
            trackers.push(LockTracker {
                thread_id: thread_id.to_string(),
                acquired_locks: Vec::new(),
                lock_times: HashMap::new(),
                waiting_for: None,
                wait_start_time: None,
                is_writer: HashMap::new(),
            });
        }

        let tracker = trackers.iter_mut().find(|t| t.thread_id == thread_id).unwrap();

        // Record waiting state
        tracker.waiting_for = Some(lock.to_string());
        tracker.wait_start_time = Some(SystemTime::now());

        // Add to waiting queue (FIFO)
        queue.entry(lock.to_string()).or_insert_with(VecDeque::new).push_back(thread_id.to_string());

        // Update contention stats
        let mut stats = self.lock_stats.lock().await;
        let stat = stats.entry(lock.to_string()).or_insert(LockStats {
            total_acquisitions: 0,
            total_holds_ms: 0,
            max_hold_ms: 0,
            contention_count: 0,
            last_acquired: None,
        });
        stat.contention_count += 1;

        Ok(())
    }

    /// Check if any locks are held too long
    pub async fn check_lock_contention(&self) -> Result<Vec<String>, String> {
        let trackers = self.trackers.lock().await;
        let mut warnings = Vec::new();

        for tracker in trackers.iter() {
            for (lock_name, acquire_time) in &tracker.lock_times {
                let elapsed = SystemTime::now()
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
        let mut seen = HashSet::new();

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
    pub async fn get_trackers_summary(&self) -> Result<Vec<(String, usize)>, String> {
        let trackers = self.trackers.lock().await;

        Ok(trackers
            .iter()
            .map(|t| (t.thread_id.clone(), t.acquired_locks.len()))
            .collect())
    }

    /// Check if a thread can acquire a lock (fairness check)
    pub async fn can_acquire_lock(&self, lock: &str, thread_id: &str) -> bool {
        let queue = self.waiting_queue.lock().await;

        if let Some(waiting_threads) = queue.get(lock) {
            // Check if this thread is first in queue
            if let Some(first_waiting) = waiting_threads.front() {
                return first_waiting == thread_id;
            }
        }

        // No queue means no contention
        true
    }

    /// Remove thread from waiting queue when lock is acquired
    pub async fn remove_from_wait_queue(&mut self, lock: &str, thread_id: &str) {
        let mut queue = self.waiting_queue.lock().await;
        if let Some(waiting_threads) = queue.get_mut(lock) {
            waiting_threads.retain(|t| t != thread_id);
        }
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

/// Test helper for simulating lock operations
pub struct LockValidatorTestHelper {
    pub validator: LockValidator,
}

impl LockValidatorTestHelper {
    /// Create a test validator with standard configuration
    pub fn new() -> Self {
        let config = LockOrderingConfig {
            lock_order: vec![
                "signal_combiner".to_string(),
                "strategy_order".to_string(),
                "strategy_metrics".to_string(),
                "traders".to_string(),
                "active_alerts".to_string(),
                "candle_builder".to_string(),
                "last_signals".to_string(),
                "open_positions".to_string(),
                "performance_profiler".to_string(),
                "system_health".to_string(),
                "trade_history".to_string(),
                "trading_metrics".to_string(),
            ],
            timeout_ms: 5000,
            max_hold_ms: 100,
            deadlock_detection_enabled: true,
        };

        Self {
            validator: LockValidator::new(config),
        }
    }

    /// Simulate a lock acquisition sequence for a thread
    pub async fn simulate_lock_sequence(&mut self, thread_id: &str, locks: &[(&str, bool)]) -> Result<(), String> {
        for (lock, is_write) in locks {
            // Record waiting first
            self.validator.record_thread_waiting(lock, thread_id).await?;

            // Check if can acquire
            if !self.validator.can_acquire_lock(lock, thread_id).await {
                return Err(format!("Thread {} cannot acquire lock {} (fairness violation)", thread_id, lock));
            }

            // Record acquisition
            self.validator.record_lock_acquisition(lock, thread_id, *is_write).await?;
            self.validator.remove_from_wait_queue(lock, thread_id).await;
        }

        Ok(())
    }

    /// Simulate releasing all locks for a thread (in reverse order)
    pub async fn simulate_release_all(&mut self, thread_id: &str) -> Result<(), String> {
        let trackers = self.validator.trackers.lock().await;
        let acquired_locks = trackers.iter()
            .find(|t| t.thread_id == thread_id)
            .map(|t| t.acquired_locks.clone())
            .unwrap_or_default();
        drop(trackers);

        // Release in reverse order
        for lock in acquired_locks.iter().rev() {
            self.validator.record_lock_release(lock, thread_id).await?;
        }

        Ok(())
    }

    /// Get current state summary
    pub async fn get_state_summary(&self) -> Result<String, String> {
        let trackers = self.validator.get_trackers_summary().await?;
        let waiting = self.validator.get_waiting_threads().await;
        let stats = self.validator.get_lock_contention_stats().await;

        let mut summary = format!("Active threads: {}\n", trackers.len());
        summary.push_str(&format!("Waiting threads: {}\n", waiting.len()));
        summary.push_str(&format!("Lock stats: {}\n", stats.len()));

        for (thread_id, lock_count) in trackers {
            summary.push_str(&format!("  Thread {}: {} locks\n", thread_id, lock_count));
        }

        for wait_info in waiting {
            summary.push_str(&format!("  Thread {} waiting for {} ({}ms)\n",
                wait_info.thread_id, wait_info.waiting_for_lock, wait_info.wait_duration_ms));
        }

        Ok(summary)
    }

    /// Record that a thread is waiting for a lock
    pub async fn record_thread_waiting(&mut self, lock: &str, thread_id: &str) -> Result<(), String> {
        self.validator.record_thread_waiting(lock, thread_id).await
    }

    /// Get information about waiting threads
    pub async fn get_waiting_threads(&self) -> Vec<ThreadWaitInfo> {
        self.validator.get_waiting_threads().await
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
            max_hold_ms: 100,
            deadlock_detection_enabled: true,
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
            max_hold_ms: 100,
            deadlock_detection_enabled: true,
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

    #[tokio::test]
    async fn test_lock_acquisition_and_release() {
        let mut helper = LockValidatorTestHelper::new();

        // Simulate thread acquiring locks
        assert!(helper.simulate_lock_sequence("thread1", &[("signal_combiner", false), ("traders", true)]).await.is_ok());

        // Check state
        let summary = helper.get_state_summary().await.unwrap();
        assert!(summary.contains("Thread thread1: 2 locks"));

        // Release locks
        assert!(helper.simulate_release_all("thread1").await.is_ok());

        // Check state after release
        let summary = helper.get_state_summary().await.unwrap();
        assert!(summary.contains("Thread thread1: 0 locks"));
    }

    #[tokio::test]
    async fn test_rwlock_semantics() {
        let validator = LockValidator::new(LockOrderingConfig {
            lock_order: vec!["test_lock".to_string()],
            timeout_ms: 5000,
            max_hold_ms: 100,
            deadlock_detection_enabled: true,
        });

        // Initially should allow both read and write
        assert!(validator.validate_rwlock_semantics("test_lock", false).await.is_ok());
        assert!(validator.validate_rwlock_semantics("test_lock", true).await.is_ok());
    }

    #[test]
    fn test_lock_order_validation_api() {
        let validator = LockValidator::new(LockOrderingConfig {
            lock_order: vec!["a".to_string(), "b".to_string(), "c".to_string()],
            timeout_ms: 5000,
            max_hold_ms: 100,
            deadlock_detection_enabled: true,
        });

        // Valid order
        assert!(validator.validate_lock_order(&["a", "b"]).is_ok());
        assert!(validator.validate_lock_order(&["a", "c"]).is_ok());

        // Invalid order
        assert!(validator.validate_lock_order(&["b", "a"]).is_err());
        assert!(validator.validate_lock_order(&["c", "a"]).is_err());
    }

    #[test]
    fn test_unknown_lock_validation() {
        let validator = LockValidator::new(LockOrderingConfig {
            lock_order: vec!["a".to_string(), "b".to_string()],
            timeout_ms: 5000,
            max_hold_ms: 100,
            deadlock_detection_enabled: true,
        });

        // Unknown lock should fail
        assert!(validator.validate_lock_order(&["unknown"]).is_err());
    }

    #[test]
    fn test_test_helper_creation() {
        let helper = LockValidatorTestHelper::new();
        assert_eq!(helper.validator.get_lock_order().len(), 12); // Standard lock order
    }
}
