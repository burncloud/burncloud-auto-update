//! 自动更新器核心实现

use crate::{UpdateConfig, UpdateError, UpdateResult};
use log::{info, error};
use self_update::backends::github;
use semver;

/// 自动更新器
pub struct AutoUpdater {
    config: UpdateConfig,
}

impl AutoUpdater {
    /// 创建新的自动更新器
    pub fn new(config: UpdateConfig) -> Self {
        Self { config }
    }

    /// 使用默认配置创建自动更新器
    pub fn with_default_config() -> Self {
        Self::new(UpdateConfig::default())
    }

    /// 检查是否有可用更新
    pub async fn check_for_updates(&self) -> UpdateResult<bool> {
        info!("检查更新中...");

        match self.check_github_updates().await {
            Ok(has_update) => Ok(has_update),
            Err(e) => {
                error!("GitHub 检查更新失败: {}", e);
                Err(e)
            }
        }
    }

    /// 执行更新
    pub async fn update_with_fallback(&self) -> UpdateResult<()> {
        info!("开始更新应用程序...");

        match self.update_from_github().await {
            Ok(_) => {
                info!("从 GitHub 更新成功");
                Ok(())
            }
            Err(e) => {
                error!("GitHub 更新失败: {}", e);
                Err(e)
            }
        }
    }

    /// 从 GitHub 检查更新
    async fn check_github_updates(&self) -> UpdateResult<bool> {
        info!("正在检查 GitHub 更新...");

        // Use ReleaseList to get the latest release information
        let target = self_update::get_target().map_err(UpdateError::from)?;
        let releases = github::ReleaseList::configure()
            .repo_owner(&self.config.github_owner)
            .repo_name(&self.config.github_repo)
            .with_target(&target)
            .build()
            .map_err(UpdateError::from)?
            .fetch()
            .map_err(UpdateError::from)?;

        if let Some(latest_release) = releases.first() {
            let current_version = &self.config.current_version;
            let release_version = latest_release.tag.trim_start_matches('v');
            let current_clean = current_version.trim_start_matches('v');

            info!("当前版本: {}, 最新版本: {}", current_version, latest_release.tag);

            // Simple string comparison or use version comparison if needed
            Ok(release_version != current_clean)
        } else {
            error!("未找到任何发布版本");
            Err(UpdateError::GitHub("未找到任何发布版本".to_string()))
        }
    }

    /// 从 GitHub 更新
    async fn update_from_github(&self) -> UpdateResult<()> {
        info!("正在从 GitHub 下载更新...");

        // Get target platform
        let target = self_update::get_target().map_err(UpdateError::from)?;

        let update = github::Update::configure()
            .map_err(UpdateError::from)?
            .repo_owner(&self.config.github_owner)
            .repo_name(&self.config.github_repo)
            .target(&target)
            .bin_name(&self.config.bin_name)
            .current_version(&self.config.current_version)
            .show_download_progress(false) // Set to true if you want progress bars
            .no_confirm(true) // Skip confirmation prompt
            .build()
            .map_err(UpdateError::from)?;

        let status = update.update()
            .map_err(UpdateError::from)?;

        match status.updated() {
            true => {
                info!("更新成功，新版本: {}", status.version());
                Ok(())
            }
            false => {
                info!("已是最新版本");
                Ok(())
            }
        }
    }

    /// 获取当前版本
    pub fn current_version(&self) -> &str {
        &self.config.current_version
    }

    /// 设置新的配置
    pub fn set_config(&mut self, config: UpdateConfig) {
        self.config = config;
    }

    /// 获取配置的引用
    pub fn config(&self) -> &UpdateConfig {
        &self.config
    }

    /// 获取手动下载链接
    pub fn get_download_links(&self) -> (String, String) {
        self.config.download_links()
    }

    /// 获取最新发布版本信息 (不执行更新)
    pub fn get_latest_release_info(&self) -> UpdateResult<Option<(String, String)>> {
        info!("获取最新发布版本信息...");

        let target = self_update::get_target().map_err(UpdateError::from)?;
        let releases = github::ReleaseList::configure()
            .repo_owner(&self.config.github_owner)
            .repo_name(&self.config.github_repo)
            .with_target(&target)
            .build()
            .map_err(UpdateError::from)?
            .fetch()
            .map_err(UpdateError::from)?;

        if let Some(latest_release) = releases.first() {
            Ok(Some((latest_release.tag.clone(), latest_release.name.clone())))
        } else {
            Ok(None)
        }
    }

    /// 检查是否需要更新 (使用版本比较)
    pub fn needs_update(&self) -> UpdateResult<bool> {
        info!("检查是否需要更新...");

        let target = self_update::get_target().map_err(UpdateError::from)?;
        let releases = github::ReleaseList::configure()
            .repo_owner(&self.config.github_owner)
            .repo_name(&self.config.github_repo)
            .with_target(&target)
            .build()
            .map_err(UpdateError::from)?
            .fetch()
            .map_err(UpdateError::from)?;

        if let Some(latest_release) = releases.first() {
            let current_version = self.config.current_version.trim_start_matches('v');
            let latest_version = latest_release.tag.trim_start_matches('v');

            // Use semver comparison if possible, otherwise string comparison
            match (semver::Version::parse(current_version), semver::Version::parse(latest_version)) {
                (Ok(current), Ok(latest)) => {
                    info!("当前版本: v{}, 最新版本: v{}", current, latest);
                    Ok(latest > current)
                }
                _ => {
                    // Fallback to string comparison
                    info!("使用字符串比较: 当前版本: {}, 最新版本: {}", current_version, latest_version);
                    Ok(latest_version != current_version)
                }
            }
        } else {
            error!("未找到任何发布版本");
            Err(UpdateError::GitHub("未找到任何发布版本".to_string()))
        }
    }

    /// 执行同步更新（避免运行时冲突）
    pub fn sync_update(&self) -> UpdateResult<()> {
        info!("开始同步更新应用程序...");

        // Get target platform
        let target = self_update::get_target().map_err(UpdateError::from)?;

        let update = github::Update::configure()
            .map_err(UpdateError::from)?
            .repo_owner(&self.config.github_owner)
            .repo_name(&self.config.github_repo)
            .target(&target)
            .bin_name(&self.config.bin_name)
            .current_version(&self.config.current_version)
            .show_download_progress(false) // Set to true if you want progress bars
            .no_confirm(true) // Skip confirmation prompt
            .build()
            .map_err(UpdateError::from)?;

        let status = update.update()
            .map_err(UpdateError::from)?;

        match status.updated() {
            true => {
                info!("更新成功，新版本: {}", status.version());
                Ok(())
            }
            false => {
                info!("已是最新版本");
                Ok(())
            }
        }
    }

    /// 同步检查更新
    pub fn sync_check_for_updates(&self) -> UpdateResult<bool> {
        info!("同步检查更新中...");

        // Use ReleaseList to get the latest release information
        let target = self_update::get_target().map_err(UpdateError::from)?;
        let releases = github::ReleaseList::configure()
            .repo_owner(&self.config.github_owner)
            .repo_name(&self.config.github_repo)
            .with_target(&target)
            .build()
            .map_err(UpdateError::from)?
            .fetch()
            .map_err(UpdateError::from)?;

        if let Some(latest_release) = releases.first() {
            let current_version = &self.config.current_version;
            let release_version = latest_release.tag.trim_start_matches('v');
            let current_clean = current_version.trim_start_matches('v');

            info!("当前版本: {}, 最新版本: {}", current_version, latest_release.tag);

            // Simple string comparison or use version comparison if needed
            Ok(release_version != current_clean)
        } else {
            error!("未找到任何发布版本");
            Err(UpdateError::GitHub("未找到任何发布版本".to_string()))
        }
    }
}

impl Clone for AutoUpdater {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_updater_creation() {
        let updater = AutoUpdater::with_default_config();
        assert_eq!(updater.current_version(), std::env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_get_download_links() {
        let updater = AutoUpdater::with_default_config();
        let (github_url, gitee_url) = updater.get_download_links();

        assert_eq!(github_url, "https://github.com/burncloud/burncloud/releases");
        assert_eq!(gitee_url, "https://gitee.com/burncloud/burncloud/releases");
    }

    #[test]
    fn test_update_config_customization() {
        let mut config = UpdateConfig::default();
        config.current_version = "1.0.0".to_string();

        let mut updater = AutoUpdater::new(config);
        assert_eq!(updater.current_version(), "1.0.0");

        let new_config = UpdateConfig {
            current_version: "2.0.0".to_string(),
            ..UpdateConfig::default()
        };

        updater.set_config(new_config);
        assert_eq!(updater.current_version(), "2.0.0");
    }

    #[test]
    fn test_clone() {
        let updater = AutoUpdater::with_default_config();
        let cloned = updater.clone();

        assert_eq!(updater.current_version(), cloned.current_version());
        assert_eq!(updater.config().github_owner, cloned.config().github_owner);
    }
}