use aws_config::BehaviorVersion;
use aws_sdk_ec2::{
    types::{AddPrefixListEntry, RemovePrefixListEntry},
    Client,
};
use clap::Parser;
use reqwest;
use std::time::Duration;
use tokio::time;
use tracing::{info, warn, error, debug};


#[derive(Parser, Debug)]
#[command(author, version, about = "Monitor external IP and update AWS VPC prefix list", long_about = None)]
struct Args {
    /// AWS region (e.g., us-east-1)
    #[arg(short, long, env = "AWS_REGION")]
    region: Option<String>,

    /// Prefix list ID to update
    #[arg(short, long, env = "PREFIX_LIST_ID")]
    prefix_list_id: String,

    /// Description for the prefix list entry
    #[arg(short, long, env = "ENTRY_DESCRIPTION", default_value = "Auto-updated host IP")]
    description: String,

    /// Check interval in seconds
    #[arg(short, long, env = "CHECK_INTERVAL", default_value = "300")]
    interval: u64,

    /// IP detection service URL
    #[arg(long, env = "IP_SERVICE_URL", default_value = "https://api.ipify.org")]
    ip_service: String,

    /// CIDR suffix (e.g., /32 for single host)
    #[arg(long, env = "CIDR_SUFFIX", default_value = "32")]
    cidr_suffix: u8,

    /// Run once and exit (for testing)
    #[arg(long, default_value = "false")]
    once: bool,
}

/// A struct representing a prefix list monitor.
///
/// This struct is used to monitor an external IP and update AWS VPC prefix list accordingly.
struct PrefixListMonitor {
    /// The client instance used to interact with the AWS EC2 service.
    client: Client,
    /// The ID of the prefix list being monitored.
    prefix_list_id: String,
    /// The description of the prefix list entry.
    description: String,
    /// The current external IP address.
    current_ip: Option<String>,
    /// The CIDR suffix used to format the IP address.
    cidr_suffix: u8,
    /// The URL of the IP service being used.
    ip_service: String,
}

impl PrefixListMonitor {
    /// Creates a new instance of `PrefixListMonitor`.
    ///
    /// # Parameters
    ///
    /// * `client`: The client instance used to interact with the AWS EC2 service.
    /// * `args`: The arguments passed to the program.
    ///
    /// # Returns
    ///
    /// A new instance of `PrefixListMonitor`.
    fn new(client: Client, args: &Args) -> Self {
        Self {
            client,
            prefix_list_id: args.prefix_list_id.clone(),
            description: args.description.clone(),
            current_ip: None,
            cidr_suffix: args.cidr_suffix,
            ip_service: args.ip_service.clone(),
        }
    }

    /// Retrieves the external IP address from the specified IP service.
    ///
    /// # Returns
    ///
    /// The external IP address as a `String`, or an error if the request fails.
    async fn get_external_ip(&self) -> Result<String, Box<dyn std::error::Error>> {
        let response = reqwest::get(&self.ip_service)
            .await?
            .text()
            .await?;
        let ip = response.trim().to_string();
        
        // Basic IP validation
        if ip.parse::<std::net::Ipv4Addr>().is_ok() {
            Ok(ip)
        } else {
            Err("Invalid IP address format".into())
        }
    }

    /// Retrieves the version of the prefix list.
    ///
    /// # Returns
    ///
    /// The version of the prefix list as an `i64`, or an error if the request fails.
    async fn get_prefix_list_version(&self) -> Result<i64, Box<dyn std::error::Error>> {
        let response = self.client
            .describe_managed_prefix_lists()
            .prefix_list_ids(&self.prefix_list_id)
            .send()
            .await?;

        let prefix_list = response
            .prefix_lists()
            .first()
            .ok_or("Prefix list not found")?;

        Ok(prefix_list.version().unwrap_or(0))
    }

    /// Retrieves the current entries from the prefix list.
    ///
    /// # Returns
    ///
    /// A vector of `String` representing the current entries in the prefix list, or an error if the request fails.
    async fn get_current_entries(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let response = self.client
            .get_managed_prefix_list_entries()
            .prefix_list_id(&self.prefix_list_id)
            .send()
            .await?;

        let entries: Vec<String> = response
            .entries()
            .iter()
            .filter_map(|e| {
                if e.description() == Some(&self.description) {
                    e.cidr().map(|s| s.to_string())
                } else {
                    None
                }
            })
            .collect();

        Ok(entries)
    }

    /// Updates the prefix list by adding or replacing entries.
    ///
    /// # Parameters
    ///
    /// * `new_cidr`: The new CIDR format of the IP address.
    /// * `old_cidrs`: A vector of old CIDRs to be removed from the prefix list.
    ///
    /// # Returns
    ///
    /// An error if the request fails, or `Ok(())` on success.
    async fn update_prefix_list(&self, new_cidr: &str, old_cidrs: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
        let version = self.get_prefix_list_version().await?;
        
        let mut modify_request = self.client
            .modify_managed_prefix_list()
            .prefix_list_id(&self.prefix_list_id)
            .current_version(version);

        // Remove old entries with matching description
        for old_cidr in &old_cidrs {
            debug!("Removing old entry: {}", old_cidr);
            let entry = RemovePrefixListEntry::builder()
                .cidr(old_cidr)
                .build();
            modify_request = modify_request.remove_entries(entry);
        }

        // Add new entry
        debug!("Adding new entry: {}", new_cidr);
        let entry = AddPrefixListEntry::builder()
            .cidr(new_cidr)
            .description(&self.description)
            .build();
        modify_request = modify_request.add_entries(entry);

        let response = modify_request.send().await?;

        if let Some(updated_list) = response.prefix_list() {
            info!(
                "Successfully updated prefix list to version {}",
                updated_list.version().unwrap_or(0)
            );
        }

        Ok(())
    }

    /// Checks the IP address and updates the prefix list accordingly.
    ///
    /// # Returns
    ///
    /// `Ok(true)` if the IP address has changed, or `Ok(false)` if it hasn't.
    /// An error if the request fails.
    async fn check_and_update(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        // Get current external IP
        let external_ip = self.get_external_ip().await?;
        let new_cidr = format!("{}/{}", external_ip, self.cidr_suffix);

        debug!("Detected external IP: {}", external_ip);

        // Check if IP has changed
        if let Some(ref current) = self.current_ip {
            if current == &external_ip {
                debug!("IP unchanged: {}", external_ip);
                return Ok(false);
            }
        }

        info!("IP change detected: {} -> {}", 
              self.current_ip.as_deref().unwrap_or("none"), 
              external_ip);

        // Get current entries from prefix list with our description
        let current_entries = self.get_current_entries().await?;

        // Check if the new CIDR is already in the list
        if current_entries.contains(&new_cidr) {
            info!("CIDR {} already exists in prefix list", new_cidr);
            self.current_ip = Some(external_ip);
            return Ok(false);
        }

        // Update prefix list
        if !current_entries.is_empty() {
            info!("Replacing {} old entries with new CIDR {}", 
                  current_entries.len(), new_cidr);
        } else {
            info!("Adding new CIDR {} to prefix list", new_cidr);
        }

        self.update_prefix_list(&new_cidr, current_entries).await?;
        self.current_ip = Some(external_ip);

        Ok(true)
    }

    /// Runs the program in a loop until stopped.
    ///
    /// # Parameters
    ///
    /// * `interval`: The check interval in seconds.
    /// * `once`: Whether to run once and exit.
    ///
    /// # Returns
    ///
    /// An error if the request fails, or `Ok(())` on success.
    async fn run(&mut self, interval: Duration, once: bool) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting prefix list monitor");
        info!("Prefix List ID: {}", self.prefix_list_id);
        info!("Description: {}", self.description);
        info!("Check interval: {}s", interval.as_secs());
        info!("IP service: {}", self.ip_service);

        loop {
            match self.check_and_update().await {
                Ok(updated) => {
                    if updated {
                        info!("✓ Prefix list updated successfully");
                    }
                }
                Err(e) => {
                    error!("Error during check: {}", e);
                }
            }

            if once {
                info!("Running in once mode, exiting");
                break;
            }

            time::sleep(interval).await;
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into())
        )
        .init();

    let args = Args::parse();

    // Load AWS config
    let config = if let Some(region) = &args.region {
        aws_config::defaults(BehaviorVersion::v2025_08_07())
            .region(aws_config::Region::new(region.clone()))
            .load()
            .await
    } else {
        aws_config::load_defaults(BehaviorVersion::v2025_08_07()).await
    };

    let client = Client::new(&config);
    let interval = Duration::from_secs(args.interval);
    let once = args.once;

    let mut monitor = PrefixListMonitor::new(client, &args);
    
    monitor.run(interval, once).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cidr_format() {
        let ip = "203.0.113.1";
        let suffix = 32;
        let cidr = format!("{}/{}", ip, suffix);
        assert_eq!(cidr, "203.0.113.1/32");
    }

    #[test]
    fn test_ip_validation() {
        assert!("192.168.1.1".parse::<std::net::Ipv4Addr>().is_ok());
        assert!("invalid".parse::<std::net::Ipv4Addr>().is_err());
    }
}