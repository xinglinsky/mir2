//! 重连策略

use std::time::Duration;

/// 重连策略
///
/// 控制网络断开后的重连行为
#[derive(Clone, Debug)]
pub struct ReconnectStrategy {
    /// 最大重连次数（0 表示不限制）
    max_attempts: u32,
    
    /// 当前重连次数
    current_attempts: u32,
    
    /// 初始延迟（毫秒）
    initial_delay_ms: u64,
    
    /// 最大延迟（毫秒）
    max_delay_ms: u64,
    
    /// 延迟增长因子（指数退避）
    backoff_factor: f64,
}

impl Default for ReconnectStrategy {
    fn default() -> Self {
        Self {
            max_attempts: 10,
            current_attempts: 0,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_factor: 1.5,
        }
    }
}

impl ReconnectStrategy {
    /// 创建新的重连策略
    ///
    /// # Arguments
    ///
    /// * `max_attempts` - 最大重连次数（0 表示不限制）
    /// * `initial_delay_ms` - 初始延迟（毫秒）
    /// * `max_delay_ms` - 最大延迟（毫秒）
    /// * `backoff_factor` - 延迟增长因子
    pub fn new(
        max_attempts: u32,
        initial_delay_ms: u64,
        max_delay_ms: u64,
        backoff_factor: f64,
    ) -> Self {
        Self {
            max_attempts,
            current_attempts: 0,
            initial_delay_ms,
            max_delay_ms,
            backoff_factor,
        }
    }

    /// 禁用重连
    pub fn disabled() -> Self {
        Self {
            max_attempts: 0,
            current_attempts: 0,
            initial_delay_ms: 0,
            max_delay_ms: 0,
            backoff_factor: 1.0,
        }
    }

    /// 记录一次重连尝试
    pub fn record_attempt(&mut self) {
        self.current_attempts += 1;
    }

    /// 检查是否应该重连
    pub fn should_reconnect(&self) -> bool {
        self.max_attempts == 0 || self.current_attempts < self.max_attempts
    }

    /// 获取下一次重连的延迟时间
    pub fn next_delay(&self) -> Duration {
        if self.current_attempts == 0 {
            return Duration::from_millis(self.initial_delay_ms);
        }

        let delay_ms = (self.initial_delay_ms as f64
            * self.backoff_factor.powi(self.current_attempts as i32 - 1))
            .min(self.max_delay_ms as f64) as u64;

        Duration::from_millis(delay_ms)
    }

    /// 重置重连计数（连接成功后调用）
    pub fn reset(&mut self) {
        self.current_attempts = 0;
    }

    /// 获取当前重连次数
    pub fn current_attempts(&self) -> u32 {
        self.current_attempts
    }
}

