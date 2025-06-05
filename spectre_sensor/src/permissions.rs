//! Platform-specific camera permission handling
//!
//! This module provides cross-platform camera permission checking and guidance
//! for users to ensure proper camera access across Windows, macOS, and Linux.

use crate::sensor::SensorError;

/// Check camera permissions for the current platform
pub async fn check_camera_permissions() -> Result<(), SensorError> {
    #[cfg(target_os = "macos")]
    {
        check_macos_camera_permission().await
    }
    #[cfg(target_os = "windows")]
    {
        check_windows_camera_permission().await
    }
    #[cfg(target_os = "linux")]
    {
        check_linux_camera_permission().await
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Ok(()) // Assume permissions are OK on other platforms
    }
}

#[cfg(target_os = "macos")]
async fn check_macos_camera_permission() -> Result<(), SensorError> {
    // Note: This would require platform-specific dependencies for full implementation
    // For now, provide clear guidance to users
    tracing::warn!("macOS camera permissions not automatically checked");
    tracing::info!("Ensure camera permissions are granted in System Preferences > Security & Privacy > Camera");
    tracing::info!("If camera access fails, check that your application has camera permissions");
    Ok(())
}

#[cfg(target_os = "windows")]
async fn check_windows_camera_permission() -> Result<(), SensorError> {
    tracing::warn!("Windows camera permissions not automatically checked");
    tracing::info!("Ensure camera access is enabled in Settings > Privacy > Camera");
    tracing::info!("If camera access fails, check Windows privacy settings for camera access");
    Ok(())
}

#[cfg(target_os = "linux")]
async fn check_linux_camera_permission() -> Result<(), SensorError> {
    // Check if user is in video group
    match std::process::Command::new("groups").output() {
        Ok(output) => {
            let groups = String::from_utf8_lossy(&output.stdout);
            if !groups.contains("video") {
                tracing::warn!("User may not be in 'video' group for camera access");
                tracing::info!("Run: sudo usermod -a -G video $USER");
                tracing::info!("Then log out and log back in for changes to take effect");
            } else {
                tracing::debug!("User is in 'video' group - camera permissions should be OK");
            }
        },
        Err(e) => {
            tracing::warn!("Failed to check user groups: {}", e);
            tracing::info!("Ensure user has camera access permissions");
        }
    }
    
    Ok(())
}

/// Provide platform-specific camera troubleshooting guidance
pub fn provide_camera_troubleshooting_guidance() {
    tracing::info!("Camera Troubleshooting Guide:");
    
    #[cfg(target_os = "windows")]
    {
        tracing::info!("Windows:");
        tracing::info!("  1. Check Settings > Privacy > Camera");
        tracing::info!("  2. Ensure 'Allow apps to access your camera' is enabled");
        tracing::info!("  3. Ensure this application is allowed camera access");
        tracing::info!("  4. Try different camera backends (DirectShow, MSMF)");
    }
    
    #[cfg(target_os = "macos")]
    {
        tracing::info!("macOS:");
        tracing::info!("  1. Check System Preferences > Security & Privacy > Camera");
        tracing::info!("  2. Ensure this application is checked in the camera access list");
        tracing::info!("  3. If not listed, try accessing camera to trigger permission prompt");
        tracing::info!("  4. Restart application after granting permissions");
    }
    
    #[cfg(target_os = "linux")]
    {
        tracing::info!("Linux:");
        tracing::info!("  1. Ensure user is in 'video' group: groups | grep video");
        tracing::info!("  2. If not: sudo usermod -a -G video $USER");
        tracing::info!("  3. Log out and log back in");
        tracing::info!("  4. Check camera device exists: ls /dev/video*");
        tracing::info!("  5. Test camera access: v4l2-ctl --list-devices");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_camera_permission_check() {
        // This should not fail on any platform
        let result = check_camera_permissions().await;
        assert!(result.is_ok(), "Camera permission check should not fail");
    }

    #[test]
    fn test_troubleshooting_guidance() {
        // This should not panic
        provide_camera_troubleshooting_guidance();
    }
}
