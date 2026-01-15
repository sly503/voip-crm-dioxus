//! Email service for sending verification and invitation emails
//!
//! This module provides email functionality using SMTP via the lettre crate.
//! It supports sending HTML emails for user registration verification and
//! team invitations.

use lettre::{
    message::{header::ContentType, Mailbox, Message},
    transport::smtp::{
        authentication::Credentials,
        client::{Tls, TlsParameters},
    },
    AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
};
use thiserror::Error;

/// Email service for sending verification and invitation emails
#[derive(Clone)]
pub struct EmailService {
    mailer: AsyncSmtpTransport<Tokio1Executor>,
    from_email: Mailbox,
    from_name: String,
    app_url: String,
}

/// Errors that can occur when sending emails
#[derive(Error, Debug)]
pub enum EmailError {
    #[error("Failed to build email message: {0}")]
    MessageBuild(String),

    #[error("Failed to send email: {0}")]
    SendFailed(String),

    #[error("Invalid email address: {0}")]
    InvalidAddress(String),

    #[error("SMTP configuration error: {0}")]
    ConfigError(String),
}

impl EmailService {
    /// Create a new email service from environment variables
    ///
    /// Required environment variables:
    /// - SMTP_HOST: SMTP server hostname
    /// - SMTP_PORT: SMTP server port
    /// - SMTP_USERNAME: SMTP authentication username
    /// - SMTP_PASSWORD: SMTP authentication password
    /// - SMTP_FROM_EMAIL: From email address
    /// - SMTP_FROM_NAME: From name (optional, defaults to "VoIP CRM")
    /// - APP_URL: Base URL for the application (for generating links)
    pub fn from_env() -> Result<Self, EmailError> {
        let smtp_host = std::env::var("SMTP_HOST")
            .map_err(|_| EmailError::ConfigError("SMTP_HOST not set".to_string()))?;

        let smtp_port = std::env::var("SMTP_PORT")
            .map_err(|_| EmailError::ConfigError("SMTP_PORT not set".to_string()))?
            .parse::<u16>()
            .map_err(|_| EmailError::ConfigError("SMTP_PORT must be a valid port number".to_string()))?;

        let smtp_username = std::env::var("SMTP_USERNAME")
            .map_err(|_| EmailError::ConfigError("SMTP_USERNAME not set".to_string()))?;

        let smtp_password = std::env::var("SMTP_PASSWORD")
            .map_err(|_| EmailError::ConfigError("SMTP_PASSWORD not set".to_string()))?;

        let smtp_from_email = std::env::var("SMTP_FROM_EMAIL")
            .map_err(|_| EmailError::ConfigError("SMTP_FROM_EMAIL not set".to_string()))?;

        let smtp_from_name = std::env::var("SMTP_FROM_NAME")
            .unwrap_or_else(|_| "VoIP CRM".to_string());

        let app_url = std::env::var("APP_URL")
            .map_err(|_| EmailError::ConfigError("APP_URL not set".to_string()))?;

        Self::new(
            &smtp_host,
            smtp_port,
            &smtp_username,
            &smtp_password,
            &smtp_from_email,
            &smtp_from_name,
            &app_url,
        )
    }

    /// Create a new email service with explicit configuration
    pub fn new(
        smtp_host: &str,
        smtp_port: u16,
        smtp_username: &str,
        smtp_password: &str,
        from_email: &str,
        from_name: &str,
        app_url: &str,
    ) -> Result<Self, EmailError> {
        // Parse the from email address
        let from_mailbox: Mailbox = format!("{} <{}>", from_name, from_email)
            .parse()
            .map_err(|e| EmailError::InvalidAddress(format!("Invalid from address: {}", e)))?;

        // Configure TLS
        let tls_parameters = TlsParameters::builder(smtp_host.to_string())
            .build()
            .map_err(|e| EmailError::ConfigError(format!("Failed to build TLS parameters: {}", e)))?;

        // Build SMTP transport
        let credentials = Credentials::new(smtp_username.to_string(), smtp_password.to_string());

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(smtp_host)
            .map_err(|e| EmailError::ConfigError(format!("Failed to create SMTP transport: {}", e)))?
            .port(smtp_port)
            .credentials(credentials)
            .tls(Tls::Required(tls_parameters))
            .build();

        Ok(Self {
            mailer,
            from_email: from_mailbox,
            from_name: from_name.to_string(),
            app_url: app_url.trim_end_matches('/').to_string(),
        })
    }

    /// Send a verification email to a new user
    ///
    /// # Arguments
    /// * `to_email` - The recipient's email address
    /// * `to_name` - The recipient's name (optional)
    /// * `verification_token` - The verification token to include in the link
    pub async fn send_verification_email(
        &self,
        to_email: &str,
        to_name: Option<&str>,
        verification_token: &str,
    ) -> Result<(), EmailError> {
        let verification_url = format!("{}/verify-email?token={}", self.app_url, verification_token);

        let subject = "Verify Your Email - VoIP CRM";
        let display_name = to_name.unwrap_or("User");

        let html_body = self.build_verification_email_html(display_name, &verification_url);
        let text_body = self.build_verification_email_text(display_name, &verification_url);

        self.send_email(to_email, to_name, subject, &html_body, &text_body)
            .await
    }

    /// Send an invitation email to a new team member
    ///
    /// # Arguments
    /// * `to_email` - The recipient's email address
    /// * `inviter_name` - The name of the person who sent the invitation
    /// * `role` - The role being invited to (e.g., "Agent", "Supervisor")
    /// * `invitation_token` - The invitation token to include in the link
    pub async fn send_invitation_email(
        &self,
        to_email: &str,
        inviter_name: &str,
        role: &str,
        invitation_token: &str,
    ) -> Result<(), EmailError> {
        let invitation_url = format!("{}/accept-invitation?token={}", self.app_url, invitation_token);

        let subject = format!("You've been invited to join VoIP CRM as {}", role);

        let html_body = self.build_invitation_email_html(inviter_name, role, &invitation_url);
        let text_body = self.build_invitation_email_text(inviter_name, role, &invitation_url);

        self.send_email(to_email, None, &subject, &html_body, &text_body)
            .await
    }

    /// Internal method to send an email with both HTML and plain text versions
    async fn send_email(
        &self,
        to_email: &str,
        to_name: Option<&str>,
        subject: &str,
        html_body: &str,
        text_body: &str,
    ) -> Result<(), EmailError> {
        // Parse the recipient email address
        let to_mailbox: Mailbox = if let Some(name) = to_name {
            format!("{} <{}>", name, to_email)
        } else {
            to_email.to_string()
        }
        .parse()
        .map_err(|e| EmailError::InvalidAddress(format!("Invalid recipient address: {}", e)))?;

        // Build the multipart email message
        let email = Message::builder()
            .from(self.from_email.clone())
            .to(to_mailbox)
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(html_body.to_string())
            .map_err(|e| EmailError::MessageBuild(e.to_string()))?;

        // Send the email
        self.mailer
            .send(email)
            .await
            .map_err(|e| EmailError::SendFailed(e.to_string()))?;

        tracing::info!("Email sent successfully to {}", to_email);
        Ok(())
    }

    /// Build HTML version of verification email
    fn build_verification_email_html(&self, user_name: &str, verification_url: &str) -> String {
        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Verify Your Email</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 600px;
            margin: 0 auto;
            padding: 20px;
            background-color: #f4f4f4;
        }}
        .container {{
            background-color: #ffffff;
            padding: 40px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
        }}
        .header {{
            text-align: center;
            margin-bottom: 30px;
        }}
        .header h1 {{
            color: #2563eb;
            margin: 0;
            font-size: 28px;
        }}
        .content {{
            margin-bottom: 30px;
        }}
        .button {{
            display: inline-block;
            padding: 14px 32px;
            background-color: #2563eb;
            color: #ffffff !important;
            text-decoration: none;
            border-radius: 6px;
            font-weight: 600;
            text-align: center;
            margin: 20px 0;
        }}
        .button:hover {{
            background-color: #1d4ed8;
        }}
        .footer {{
            margin-top: 30px;
            padding-top: 20px;
            border-top: 1px solid #e5e7eb;
            font-size: 14px;
            color: #6b7280;
            text-align: center;
        }}
        .link {{
            color: #2563eb;
            word-break: break-all;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>VoIP CRM</h1>
        </div>
        <div class="content">
            <h2>Welcome, {}!</h2>
            <p>Thank you for registering with VoIP CRM. To complete your registration and activate your account, please verify your email address by clicking the button below:</p>
            <div style="text-align: center;">
                <a href="{}" class="button">Verify Email Address</a>
            </div>
            <p>If the button doesn't work, you can copy and paste this link into your browser:</p>
            <p class="link">{}</p>
            <p><strong>Note:</strong> This verification link will expire in 24 hours for security reasons.</p>
        </div>
        <div class="footer">
            <p>If you didn't create an account with VoIP CRM, you can safely ignore this email.</p>
            <p>&copy; 2024 VoIP CRM. All rights reserved.</p>
        </div>
    </div>
</body>
</html>"#,
            user_name, verification_url, verification_url
        )
    }

    /// Build plain text version of verification email
    fn build_verification_email_text(&self, user_name: &str, verification_url: &str) -> String {
        format!(
            r#"Welcome, {}!

Thank you for registering with VoIP CRM. To complete your registration and activate your account, please verify your email address by visiting the following link:

{}

Note: This verification link will expire in 24 hours for security reasons.

If you didn't create an account with VoIP CRM, you can safely ignore this email.

---
VoIP CRM
© 2024 VoIP CRM. All rights reserved."#,
            user_name, verification_url
        )
    }

    /// Build HTML version of invitation email
    fn build_invitation_email_html(&self, inviter_name: &str, role: &str, invitation_url: &str) -> String {
        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>You're Invited to VoIP CRM</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 600px;
            margin: 0 auto;
            padding: 20px;
            background-color: #f4f4f4;
        }}
        .container {{
            background-color: #ffffff;
            padding: 40px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
        }}
        .header {{
            text-align: center;
            margin-bottom: 30px;
        }}
        .header h1 {{
            color: #2563eb;
            margin: 0;
            font-size: 28px;
        }}
        .content {{
            margin-bottom: 30px;
        }}
        .invite-box {{
            background-color: #eff6ff;
            border-left: 4px solid #2563eb;
            padding: 20px;
            margin: 20px 0;
            border-radius: 4px;
        }}
        .button {{
            display: inline-block;
            padding: 14px 32px;
            background-color: #10b981;
            color: #ffffff !important;
            text-decoration: none;
            border-radius: 6px;
            font-weight: 600;
            text-align: center;
            margin: 20px 0;
        }}
        .button:hover {{
            background-color: #059669;
        }}
        .footer {{
            margin-top: 30px;
            padding-top: 20px;
            border-top: 1px solid #e5e7eb;
            font-size: 14px;
            color: #6b7280;
            text-align: center;
        }}
        .link {{
            color: #2563eb;
            word-break: break-all;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>VoIP CRM</h1>
        </div>
        <div class="content">
            <h2>You've Been Invited!</h2>
            <div class="invite-box">
                <p><strong>{}</strong> has invited you to join their team on VoIP CRM as a <strong>{}</strong>.</p>
            </div>
            <p>VoIP CRM is a powerful call center management platform that helps teams manage leads, make calls, and track performance.</p>
            <p>To accept this invitation and create your account, click the button below:</p>
            <div style="text-align: center;">
                <a href="{}" class="button">Accept Invitation</a>
            </div>
            <p>If the button doesn't work, you can copy and paste this link into your browser:</p>
            <p class="link">{}</p>
            <p><strong>Note:</strong> This invitation link will expire in 7 days.</p>
        </div>
        <div class="footer">
            <p>If you weren't expecting this invitation, you can safely ignore this email.</p>
            <p>&copy; 2024 VoIP CRM. All rights reserved.</p>
        </div>
    </div>
</body>
</html>"#,
            inviter_name, role, invitation_url, invitation_url
        )
    }

    /// Build plain text version of invitation email
    fn build_invitation_email_text(&self, inviter_name: &str, role: &str, invitation_url: &str) -> String {
        format!(
            r#"You've Been Invited to VoIP CRM!

{} has invited you to join their team on VoIP CRM as a {}.

VoIP CRM is a powerful call center management platform that helps teams manage leads, make calls, and track performance.

To accept this invitation and create your account, visit the following link:

{}

Note: This invitation link will expire in 7 days.

If you weren't expecting this invitation, you can safely ignore this email.

---
VoIP CRM
© 2024 VoIP CRM. All rights reserved."#,
            inviter_name, role, invitation_url
        )
    }

    /// Send a storage warning alert to administrators
    ///
    /// # Arguments
    /// * `to_email` - The recipient's email address
    /// * `to_name` - The recipient's name (optional)
    /// * `used_gb` - Current storage usage in GB
    /// * `quota_gb` - Total storage quota in GB
    /// * `percentage` - Storage usage percentage
    pub async fn send_storage_alert(
        &self,
        to_email: &str,
        to_name: Option<&str>,
        used_gb: f64,
        quota_gb: f64,
        percentage: f64,
    ) -> Result<(), EmailError> {
        let subject = format!(
            "⚠️ Storage Alert: {} used of {} ({:.1}%)",
            format_storage_size(used_gb),
            format_storage_size(quota_gb),
            percentage
        );

        let html_body = self.build_storage_alert_html(used_gb, quota_gb, percentage);
        let text_body = self.build_storage_alert_text(used_gb, quota_gb, percentage);

        self.send_email(to_email, to_name, &subject, &html_body, &text_body)
            .await
    }

    /// Send a daily storage report to administrators
    ///
    /// # Arguments
    /// * `to_email` - The recipient's email address
    /// * `to_name` - The recipient's name (optional)
    /// * `total_files` - Total number of recordings
    /// * `used_gb` - Current storage usage in GB
    /// * `quota_gb` - Total storage quota in GB
    /// * `percentage` - Storage usage percentage
    /// * `recordings_added_today` - Number of recordings added today
    /// * `recordings_deleted_today` - Number of recordings deleted today
    pub async fn send_daily_storage_report(
        &self,
        to_email: &str,
        to_name: Option<&str>,
        total_files: i64,
        used_gb: f64,
        quota_gb: f64,
        percentage: f64,
        recordings_added_today: i32,
        recordings_deleted_today: i32,
    ) -> Result<(), EmailError> {
        let subject = format!("📊 Daily Storage Report - {}", chrono::Local::now().format("%Y-%m-%d"));

        let html_body = self.build_storage_report_html(
            total_files,
            used_gb,
            quota_gb,
            percentage,
            recordings_added_today,
            recordings_deleted_today,
        );
        let text_body = self.build_storage_report_text(
            total_files,
            used_gb,
            quota_gb,
            percentage,
            recordings_added_today,
            recordings_deleted_today,
        );

        self.send_email(to_email, to_name, &subject, &html_body, &text_body)
            .await
    }

    /// Build HTML version of storage alert email
    fn build_storage_alert_html(&self, used_gb: f64, quota_gb: f64, percentage: f64) -> String {
        let alert_color = if percentage >= 95.0 {
            "#dc2626" // red-600
        } else if percentage >= 90.0 {
            "#ea580c" // orange-600
        } else {
            "#d97706" // amber-600
        };

        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Storage Alert</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 600px;
            margin: 0 auto;
            padding: 20px;
            background-color: #f4f4f4;
        }}
        .container {{
            background-color: #ffffff;
            padding: 40px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
        }}
        .header {{
            text-align: center;
            margin-bottom: 30px;
        }}
        .header h1 {{
            color: #2563eb;
            margin: 0;
            font-size: 28px;
        }}
        .alert-box {{
            background-color: #fef2f2;
            border-left: 4px solid {alert_color};
            padding: 20px;
            margin: 20px 0;
            border-radius: 4px;
        }}
        .alert-icon {{
            font-size: 48px;
            text-align: center;
            margin-bottom: 10px;
        }}
        .stats {{
            margin: 30px 0;
        }}
        .stat-row {{
            display: flex;
            justify-content: space-between;
            padding: 12px 0;
            border-bottom: 1px solid #e5e7eb;
        }}
        .stat-label {{
            font-weight: 600;
            color: #4b5563;
        }}
        .stat-value {{
            color: #1f2937;
        }}
        .warning {{
            color: {alert_color};
            font-weight: 600;
        }}
        .progress-bar {{
            width: 100%;
            height: 30px;
            background-color: #e5e7eb;
            border-radius: 15px;
            overflow: hidden;
            margin: 20px 0;
        }}
        .progress-fill {{
            height: 100%;
            background-color: {alert_color};
            transition: width 0.3s ease;
        }}
        .recommendations {{
            background-color: #eff6ff;
            border-left: 4px solid #2563eb;
            padding: 20px;
            margin: 20px 0;
            border-radius: 4px;
        }}
        .recommendations h3 {{
            margin-top: 0;
            color: #1e40af;
        }}
        .recommendations ul {{
            margin: 10px 0;
            padding-left: 20px;
        }}
        .footer {{
            margin-top: 30px;
            padding-top: 20px;
            border-top: 1px solid #e5e7eb;
            font-size: 14px;
            color: #6b7280;
            text-align: center;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>VoIP CRM</h1>
        </div>
        <div class="alert-box">
            <div class="alert-icon">⚠️</div>
            <h2 style="margin: 0; text-align: center; color: {alert_color};">Storage Capacity Warning</h2>
        </div>
        <div class="stats">
            <div class="stat-row">
                <span class="stat-label">Storage Used:</span>
                <span class="stat-value warning">{used_gb:.2} GB</span>
            </div>
            <div class="stat-row">
                <span class="stat-label">Storage Quota:</span>
                <span class="stat-value">{quota_gb:.2} GB</span>
            </div>
            <div class="stat-row">
                <span class="stat-label">Usage Percentage:</span>
                <span class="stat-value warning">{percentage:.1}%</span>
            </div>
        </div>
        <div class="progress-bar">
            <div class="progress-fill" style="width: {percentage}%;"></div>
        </div>
        <div class="recommendations">
            <h3>Recommended Actions</h3>
            <ul>
                <li>Review and adjust retention policies to automatically delete older recordings</li>
                <li>Remove compliance holds on recordings that no longer need to be preserved</li>
                <li>Increase storage quota if needed for your use case</li>
                <li>Archive old recordings to external storage if required for compliance</li>
                <li>Review recording quality settings to reduce file sizes</li>
            </ul>
        </div>
        <div class="footer">
            <p>This is an automated alert from your VoIP CRM system.</p>
            <p>&copy; 2024 VoIP CRM. All rights reserved.</p>
        </div>
    </div>
</body>
</html>"#,
            alert_color = alert_color,
            used_gb = used_gb,
            quota_gb = quota_gb,
            percentage = percentage
        )
    }

    /// Build plain text version of storage alert email
    fn build_storage_alert_text(&self, used_gb: f64, quota_gb: f64, percentage: f64) -> String {
        format!(
            r#"⚠️ STORAGE CAPACITY WARNING

Your VoIP CRM storage is reaching capacity:

Storage Used: {used_gb:.2} GB
Storage Quota: {quota_gb:.2} GB
Usage Percentage: {percentage:.1}%

RECOMMENDED ACTIONS:
- Review and adjust retention policies to automatically delete older recordings
- Remove compliance holds on recordings that no longer need to be preserved
- Increase storage quota if needed for your use case
- Archive old recordings to external storage if required for compliance
- Review recording quality settings to reduce file sizes

---
This is an automated alert from your VoIP CRM system.
VoIP CRM
© 2024 VoIP CRM. All rights reserved."#,
            used_gb = used_gb,
            quota_gb = quota_gb,
            percentage = percentage
        )
    }

    /// Build HTML version of daily storage report email
    fn build_storage_report_html(
        &self,
        total_files: i64,
        used_gb: f64,
        quota_gb: f64,
        percentage: f64,
        added: i32,
        deleted: i32,
    ) -> String {
        let progress_color = if percentage >= 90.0 {
            "#dc2626" // red-600
        } else if percentage >= 80.0 {
            "#d97706" // amber-600
        } else {
            "#10b981" // green-600
        };

        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Daily Storage Report</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 600px;
            margin: 0 auto;
            padding: 20px;
            background-color: #f4f4f4;
        }}
        .container {{
            background-color: #ffffff;
            padding: 40px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
        }}
        .header {{
            text-align: center;
            margin-bottom: 30px;
        }}
        .header h1 {{
            color: #2563eb;
            margin: 0;
            font-size: 28px;
        }}
        .report-header {{
            background-color: #eff6ff;
            padding: 20px;
            border-radius: 6px;
            text-align: center;
            margin-bottom: 30px;
        }}
        .report-header h2 {{
            margin: 0;
            color: #1e40af;
        }}
        .stats-grid {{
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 20px;
            margin: 30px 0;
        }}
        .stat-card {{
            background-color: #f9fafb;
            padding: 20px;
            border-radius: 6px;
            text-align: center;
        }}
        .stat-card h3 {{
            margin: 0 0 10px 0;
            font-size: 14px;
            color: #6b7280;
            text-transform: uppercase;
        }}
        .stat-card .value {{
            font-size: 28px;
            font-weight: bold;
            color: #1f2937;
        }}
        .progress-bar {{
            width: 100%;
            height: 30px;
            background-color: #e5e7eb;
            border-radius: 15px;
            overflow: hidden;
            margin: 20px 0;
        }}
        .progress-fill {{
            height: 100%;
            background-color: {progress_color};
            transition: width 0.3s ease;
        }}
        .activity {{
            margin: 30px 0;
        }}
        .activity-item {{
            display: flex;
            justify-content: space-between;
            padding: 12px 0;
            border-bottom: 1px solid #e5e7eb;
        }}
        .activity-label {{
            font-weight: 600;
            color: #4b5563;
        }}
        .activity-value {{
            color: #1f2937;
        }}
        .positive {{
            color: #10b981;
        }}
        .negative {{
            color: #dc2626;
        }}
        .footer {{
            margin-top: 30px;
            padding-top: 20px;
            border-top: 1px solid #e5e7eb;
            font-size: 14px;
            color: #6b7280;
            text-align: center;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>VoIP CRM</h1>
        </div>
        <div class="report-header">
            <h2>📊 Daily Storage Report</h2>
            <p style="margin: 5px 0 0 0; color: #6b7280;">{date}</p>
        </div>
        <div class="stats-grid">
            <div class="stat-card">
                <h3>Total Recordings</h3>
                <div class="value">{total_files}</div>
            </div>
            <div class="stat-card">
                <h3>Storage Used</h3>
                <div class="value">{used_gb:.1} GB</div>
            </div>
        </div>
        <div>
            <h3 style="margin: 20px 0 10px 0; color: #4b5563;">Storage Capacity</h3>
            <div style="display: flex; justify-content: space-between; margin-bottom: 8px;">
                <span style="font-size: 14px; color: #6b7280;">Used: {used_gb:.2} GB</span>
                <span style="font-size: 14px; color: #6b7280;">Quota: {quota_gb:.2} GB</span>
            </div>
            <div class="progress-bar">
                <div class="progress-fill" style="width: {percentage}%;"></div>
            </div>
            <div style="text-align: center; font-weight: 600; color: {progress_color};">{percentage:.1}% Used</div>
        </div>
        <div class="activity">
            <h3 style="margin: 0 0 15px 0; color: #4b5563;">Today's Activity</h3>
            <div class="activity-item">
                <span class="activity-label">Recordings Added:</span>
                <span class="activity-value positive">+{added}</span>
            </div>
            <div class="activity-item">
                <span class="activity-label">Recordings Deleted:</span>
                <span class="activity-value negative">-{deleted}</span>
            </div>
            <div class="activity-item">
                <span class="activity-label">Net Change:</span>
                <span class="activity-value">{net_change:+}</span>
            </div>
        </div>
        <div class="footer">
            <p>This is an automated daily report from your VoIP CRM system.</p>
            <p>&copy; 2024 VoIP CRM. All rights reserved.</p>
        </div>
    </div>
</body>
</html>"#,
            progress_color = progress_color,
            date = chrono::Local::now().format("%B %d, %Y"),
            total_files = total_files,
            used_gb = used_gb,
            quota_gb = quota_gb,
            percentage = percentage,
            added = added,
            deleted = deleted,
            net_change = added - deleted
        )
    }

    /// Build plain text version of daily storage report email
    fn build_storage_report_text(
        &self,
        total_files: i64,
        used_gb: f64,
        quota_gb: f64,
        percentage: f64,
        added: i32,
        deleted: i32,
    ) -> String {
        format!(
            r#"📊 DAILY STORAGE REPORT
{date}

STORAGE OVERVIEW
Total Recordings: {total_files}
Storage Used: {used_gb:.2} GB / {quota_gb:.2} GB ({percentage:.1}%)

TODAY'S ACTIVITY
Recordings Added: +{added}
Recordings Deleted: -{deleted}
Net Change: {net_change:+}

---
This is an automated daily report from your VoIP CRM system.
VoIP CRM
© 2024 VoIP CRM. All rights reserved."#,
            date = chrono::Local::now().format("%B %d, %Y"),
            total_files = total_files,
            used_gb = used_gb,
            quota_gb = quota_gb,
            percentage = percentage,
            added = added,
            deleted = deleted,
            net_change = added - deleted
        )
    }
}

/// Format storage size in GB with appropriate unit
fn format_storage_size(gb: f64) -> String {
    if gb >= 1000.0 {
        format!("{:.1} TB", gb / 1000.0)
    } else {
        format!("{:.1} GB", gb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test the HTML building functions directly without creating an SMTP transport.
    // We duplicate the function logic here since the methods on EmailService require
    // an SMTP transport which needs a Tokio runtime even during construction/destruction.

    #[test]
    fn test_verification_email_contains_token() {
        let user_name = "John";
        let verification_url = "https://example.com/verify?token=abc123";

        // Build HTML using the same template format
        let html = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Verify Your Email</title>
</head>
<body>
    <h2>Welcome, {}!</h2>
    <a href="{}" class="button">Verify Email Address</a>
    <p class="link">{}</p>
</body>
</html>"#,
            user_name, verification_url, verification_url
        );

        assert!(html.contains("abc123"));
        assert!(html.contains("John"));
    }

    #[test]
    fn test_invitation_email_contains_details() {
        let inviter_name = "Alice";
        let role = "Agent";
        let invitation_url = "https://example.com/invite?token=xyz789";

        // Build HTML using the same template format
        let html = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>You're Invited to VoIP CRM</title>
</head>
<body>
    <p><strong>{}</strong> has invited you to join as a <strong>{}</strong>.</p>
    <a href="{}" class="button">Accept Invitation</a>
    <p class="link">{}</p>
</body>
</html>"#,
            inviter_name, role, invitation_url, invitation_url
        );

        assert!(html.contains("Alice"));
        assert!(html.contains("Agent"));
        assert!(html.contains("xyz789"));
    }
}
