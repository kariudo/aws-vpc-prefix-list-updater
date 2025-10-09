<div align="center">

# AWS VPC Prefix List Updater

[![Rust](https://img.shields.io/badge/Rust-1.86%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**AWS VPC Prefix List Updater** is a _🔥 blazingly-fast, 🧠 memory-safe, 🔋 batteries-included, 💺ergonomic, 🦀 100% Rust-powered_ daemon that monitors your external public IP address and automatically updates an AWS VPC managed prefix list entry. Perfect for maintaining access to AWS resources from dynamic IP addresses.

_Consider keeping me caffinated:_

[![Ko-Fi](https://img.shields.io/badge/Ko--fi-F16061?style=for-the-badge&logo=ko-fi&logoColor=white)](https://ko-fi.com/kariudo)
[![BuyMeACoffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-ffdd00?style=for-the-badge&logo=buy-me-a-coffee&logoColor=black)](https://www.buymeacoffee.com/kariudo)

</div>

## 🤔 Use Case

I got really tired of having to go into the AWS console to whitelist my IP in a
prefix list every time my power at home flickered causing my fiber gateway to
give me a new IP address. So my solution... code! So I wrote this tool for myself
but you should use it too!

This tool is ideal when you need to:

- Grant your home/office network access to AWS resources (RDS, EC2, etc.) with a dynamic IP
- Maintain security group rules that reference your current IP automatically
- Run in a Docker container for easy deployment and management
- Keep a prefix list entry up-to-date without manual intervention

## 🧺 Features

- 🔄 **Automatic IP Monitoring**: Continuously checks external IP at configurable intervals
- 🎯 **Smart Updates**: Only updates AWS when IP actually changes
- 🏷️ **Description-Based Management**: Uses entry descriptions to manage only its own entries
- 🐳 **Docker Ready**: Includes Dockerfile and docker-compose setup
- 📝 **Structured Logging**: Uses tracing for detailed, filterable logs
- ⚡ **Lightweight**: Small binary (~10MB) with minimal memory footprint
- 🔒 **IAM Role Support**: Works with instance profiles, credentials, or environment variables

## 🏃🏻 Quick Start

### Using Docker Compose (Recommended)

1. **Clone and configure**:

```bash
git clone <repository>
cd aws-vpc-prefix-list-monitor
cp .env.example .env
# Edit .env with your settings
```

2. **Build and run**:

```bash
docker-compose up -d
```

3. **View logs**:

```bash
docker-compose logs -f
```

### Using Docker

```bash
# Build
docker build -t aws-prefix-monitor .

# Run
docker run -d \
  --name prefix-monitor \
  --restart unless-stopped \
  -e PREFIX_LIST_ID=pl-12345678 \
  -e AWS_REGION=us-east-1 \
  -e AWS_ACCESS_KEY_ID=your_key \
  -e AWS_SECRET_ACCESS_KEY=your_secret \
  -e CHECK_INTERVAL=300 \
  aws-prefix-monitor
```

### Building from Source

```bash
cargo build --release
./target/release/aws-vpc-prefix-list-monitor \
  --prefix-list-id pl-12345678 \
  --region us-east-1
```

## ⚙️ Configuration

### Environment Variables

| Variable                | Required | Default                | Description                                    |
| ----------------------- | -------- | ---------------------- | ---------------------------------------------- |
| `PREFIX_LIST_ID`        | Yes      | -                      | AWS managed prefix list ID (e.g., pl-12345678) |
| `AWS_REGION`            | No       | us-east-1              | AWS region                                     |
| `AWS_ACCESS_KEY_ID`     | No\*     | -                      | AWS access key                                 |
| `AWS_SECRET_ACCESS_KEY` | No\*     | -                      | AWS secret key                                 |
| `ENTRY_DESCRIPTION`     | No       | "Auto-updated host IP" | Description for managed entries                |
| `CHECK_INTERVAL`        | No       | 300                    | Seconds between IP checks                      |
| `CIDR_SUFFIX`           | No       | 32                     | CIDR suffix (32 = single host)                 |
| `IP_SERVICE_URL`        | No       | https://api.ipify.org  | IP detection service                           |
| `RUST_LOG`              | No       | info                   | Log level (trace/debug/info/warn/error)        |

\*Not required if using IAM roles/instance profiles

### Command Line Options

```bash
Options:
  -r, --region <REGION>              AWS region [env: AWS_REGION]
  -p, --prefix-list-id <ID>          Prefix list ID [env: PREFIX_LIST_ID]
  -d, --description <DESC>           Entry description [env: ENTRY_DESCRIPTION]
  -i, --interval <SECONDS>           Check interval [env: CHECK_INTERVAL]
      --ip-service <URL>             IP service URL [env: IP_SERVICE_URL]
      --cidr-suffix <BITS>           CIDR suffix [env: CIDR_SUFFIX]
      --once                         Run once and exit (for testing)
  -h, --help                         Print help
  -V, --version                      Print version
```

## 💁🏻‍♂️ How It Works

1. **IP Detection**: Queries an external service (default: ipify.org) to get current public IP
2. **Change Detection**: Compares with previously known IP
3. **Entry Lookup**: Finds existing entries in prefix list matching the configured description
4. **Update**: If IP changed, removes old entries and adds new one with updated CIDR
5. **Wait**: Sleeps for configured interval before next check

The tool only manages entries with the specific description you configure, leaving other entries untouched.

## 🔑 IAM Permissions

The AWS credentials must have these permissions:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "ec2:DescribeManagedPrefixLists",
        "ec2:GetManagedPrefixListEntries",
        "ec2:ModifyManagedPrefixList"
      ],
      "Resource": "*"
    }
  ]
}
```

For production, scope the `Resource` to specific prefix list ARNs:

```json
"Resource": "arn:aws:ec2:us-east-1:123456789012:prefix-list/pl-12345678"
```

## 🧪 Testing

Test without starting the daemon:

```bash
# Test one update cycle
docker run --rm \
  -e PREFIX_LIST_ID=pl-12345678 \
  -e AWS_REGION=us-east-1 \
  -e AWS_ACCESS_KEY_ID=your_key \
  -e AWS_SECRET_ACCESS_KEY=your_secret \
  -e RUST_LOG=debug \
  aws-prefix-monitor --once
```

Or with source build:

```bash
cargo run -- --prefix-list-id pl-12345678 --once
```

## ✅ Monitoring

### Docker Logs

```bash
docker-compose logs -f prefix-list-monitor
```

### Health Check

The container includes a health check that runs the tool in `--once` mode to verify AWS connectivity.

### Expected Log Output

```
INFO  Starting prefix list monitor
INFO  Prefix List ID: pl-12345678
INFO  Description: Auto-updated host IP
INFO  Check interval: 300s
DEBUG Detected external IP: 203.0.113.42
INFO  IP change detected: none -> 203.0.113.42
INFO  Adding new CIDR 203.0.113.42/32 to prefix list
INFO  Successfully updated prefix list to version 2
INFO  ✓ Prefix list updated successfully
```

## 👍🏻 Deployment Examples

### AWS ECS with IAM Role

```yaml
# task-definition.json
{
  "family": "prefix-list-monitor",
  "taskRoleArn": "arn:aws:iam::123456789012:role/prefix-list-updater-role",
  "containerDefinitions":
    [
      {
        "name": "monitor",
        "image": "your-registry/aws-prefix-monitor:latest",
        "environment":
          [
            { "name": "PREFIX_LIST_ID", "value": "pl-12345678" },
            { "name": "AWS_REGION", "value": "us-east-1" },
          ],
      },
    ],
}
```

### Docker on EC2 with Instance Profile

```bash
docker run -d \
  --name prefix-monitor \
  --restart unless-stopped \
  -e PREFIX_LIST_ID=pl-12345678 \
  -e AWS_REGION=us-east-1 \
  aws-prefix-monitor
```

### Kubernetes

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: prefix-list-monitor
spec:
  replicas: 1
  template:
    spec:
      serviceAccountName: prefix-list-updater # With IRSA
      containers:
        - name: monitor
          image: aws-prefix-monitor:latest
          env:
            - name: PREFIX_LIST_ID
              value: "pl-12345678"
            - name: AWS_REGION
              value: "us-east-1"
```

## 👷🏻 Troubleshooting

### Container won't start

- Check AWS credentials are set correctly
- Verify PREFIX_LIST_ID exists in your AWS account
- Check logs: `docker logs prefix-list-monitor`

### IP not updating

- Verify IAM permissions
- Check if prefix list has capacity for new entries
- Ensure no other process is modifying the same entries
- Review logs with `RUST_LOG=debug`

### "Version conflict" errors

- Another process modified the prefix list between read and write
- The tool will retry on next interval
- Consider increasing CHECK_INTERVAL if this happens frequently

## 👀 Alternative IP Services

If ipify.org is unavailable, configure alternatives:

```bash
# Using ifconfig.me
IP_SERVICE_URL=https://ifconfig.me

# Using icanhazip.com
IP_SERVICE_URL=https://icanhazip.com

# Using AWS checkip
IP_SERVICE_URL=https://checkip.amazonaws.com
```

## 🛠️ Development

Run tests:

```bash
cargo test
```

Run locally with debug logging:

```bash
RUST_LOG=debug cargo run -- \
  --prefix-list-id pl-12345678 \
  --once
```

Build optimized binary:

```bash
cargo build --release
```

## 📜 License

MIT License - see [LICENSE](LICENSE) for details.

## 🤝 Contributing

We welcome contributions! Please follow these steps:

1. Fork the repository
2. Create a feature branch (`git checkout -b feat/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feat/amazing-feature`)
5. Open a Pull Request

<p align="center">
  Made with ❤️ by <a href="https://github.com/kariudo">kariudo</a> |
  ☕ <a href="https://buymeacoffee.com/kariudo">Support the developer</a>
</p>
