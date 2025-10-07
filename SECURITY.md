# Security Guide for NZEZA Trading System

This document describes security best practices for managing sensitive data in the NZEZA trading system, including API keys, secrets, and mnemonic phrases.

## ⚠️ CRITICAL Security Requirements

### 1. API Key Strength

**All API keys MUST be at least 32 characters long** (256 bits of entropy).

The system will **PANIC** on startup if any API key is shorter than 32 characters.

Generate secure API keys:
```bash
# Generate a secure 32-character API key
openssl rand -base64 32

# Example output: 4f8jK2mN9pL3qR5tU7vW8xY0zA1bC3dE4fG6hI8jK==
```

### 2. Secret Management Options

#### Option 1: 1Password CLI (Recommended for Production)

Install 1Password CLI:
```bash
# macOS
brew install --cask 1password-cli

# Linux
# Download from https://developer.1password.com/docs/cli/get-started/
```

Store secrets in 1Password and reference them in your code:
```rust
use nzeza::secrets::{load_api_key, SecretConfig};

let config = SecretConfig {
    allow_env_vars: false,  // Disable env vars in production
    require_op_cli: true,   // Require 1Password CLI
};

let api_key = load_api_key(
    "op://Private/Coinbase/api_key",  // 1Password reference
    "COINBASE_API_KEY",                // Fallback env var name
    &config
)?;
```

#### Option 2: Environment Variables (Development Only)

**⚠️ WARNING: Environment variables are INSECURE for production use**

Environment variables:
- Are visible in process listings (`ps aux | grep nzeza`)
- Can be leaked in core dumps
- May be logged by system monitoring tools
- Persist in shell history

Only use environment variables in development.

### 3. API Authentication

**The bot will NOT start without proper API key configuration.**

```bash
# Generate a secure API key (minimum 32 characters recommended)
export API_KEYS=$(openssl rand -hex 32)

# For multiple keys, use comma-separated values
export API_KEYS=$(openssl rand -hex 32),$(openssl rand -hex 32)
```

**Never use default or weak API keys in production!**

### 2. TLS/HTTPS Encryption

⚠️ **WARNING**: The bot currently runs on HTTP only (port 3000), which transmits API keys and trading data in plaintext.

#### Production Deployment Options

**Option A: Reverse Proxy with TLS (Recommended)**

Use a reverse proxy like Nginx or Caddy to handle TLS termination:

**Nginx Configuration Example:**
```nginx
# /etc/nginx/sites-available/nzeza-bot

server {
    listen 443 ssl http2;
    server_name your-domain.com;

    # TLS certificates (use Let's Encrypt for free certificates)
    ssl_certificate /etc/letsencrypt/live/your-domain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/your-domain.com/privkey.pem;

    # Modern TLS configuration
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;
    ssl_prefer_server_ciphers on;

    # Security headers
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
    add_header X-Frame-Options "DENY" always;
    add_header X-Content-Type-Options "nosniff" always;

    # Proxy to nzeza bot
    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_cache_bypass $http_upgrade;
    }
}

# Redirect HTTP to HTTPS
server {
    listen 80;
    server_name your-domain.com;
    return 301 https://$server_name$request_uri;
}
```

**Caddy Configuration Example (Simpler):**
```caddyfile
# Caddyfile

your-domain.com {
    reverse_proxy localhost:3000

    # Caddy automatically obtains and renews TLS certificates!
}
```

Start Caddy:
```bash
sudo caddy run --config Caddyfile
```

**Option B: VPN/Private Network**

If the bot is only accessed internally:
- Deploy on a private network (VPC)
- Use VPN for access (WireGuard, OpenVPN)
- Restrict access with firewall rules

**Option C: SSH Tunnel (Development/Testing)**

For temporary secure access:
```bash
# On your local machine
ssh -L 3000:localhost:3000 user@server-ip

# Then access https://localhost:3000 locally
```

### 3. Secrets Management

**Never hardcode sensitive values in code or commit them to version control.**

#### Production Secrets Storage

**Option A: HashiCorp Vault**
```bash
# Store secrets in Vault
vault kv put secret/nzeza \
    api_keys="your-secure-key" \
    dydx_mnemonic="your mnemonic phrase" \
    initial_capital="50000"

# Retrieve in application startup script
export API_KEYS=$(vault kv get -field=api_keys secret/nzeza)
export DYDX_MNEMONIC=$(vault kv get -field=dydx_mnemonic secret/nzeza)
export INITIAL_CAPITAL=$(vault kv get -field=initial_capital secret/nzeza)
```

**Option B: AWS Secrets Manager**
```bash
# Store secret
aws secretsmanager create-secret \
    --name nzeza/api_keys \
    --secret-string "your-secure-key"

# Retrieve in startup script
export API_KEYS=$(aws secretsmanager get-secret-value \
    --secret-id nzeza/api_keys \
    --query SecretString \
    --output text)
```

**Option C: Environment Files (Minimum Security)**

If using .env files:
```bash
# .env (NEVER commit this file!)
API_KEYS=your_secure_random_key_here_min_32_chars
DYDX_MNEMONIC="your mnemonic phrase"
INITIAL_CAPITAL=50000
```

Add to `.gitignore`:
```
.env
.env.*
!.env.example
```

### 4. Mnemonic Security

The bot uses `zeroize` to clear mnemonics from memory after wallet creation, but:

⚠️ **Best Practices:**
- Use a dedicated wallet for the bot (not your main wallet)
- Fund only what you're willing to risk
- Consider using hardware wallets for production
- Rotate mnemonics periodically
- Monitor wallet activity for unauthorized transactions

### 5. Network Security

#### Firewall Configuration

Only expose necessary ports:
```bash
# UFW (Ubuntu/Debian)
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow 22/tcp    # SSH
sudo ufw allow 443/tcp   # HTTPS (if using reverse proxy)
sudo ufw enable

# Do NOT expose port 3000 directly!
```

#### IP Whitelisting

Restrict API access to known IPs (if using reverse proxy):
```nginx
# In Nginx location block
location / {
    allow 203.0.113.0/24;  # Your office network
    allow 198.51.100.42;   # Your home IP
    deny all;

    proxy_pass http://127.0.0.1:3000;
}
```

### 6. Rate Limiting

The bot includes built-in rate limiting, but add additional protection:

```nginx
# Nginx rate limiting
limit_req_zone $binary_remote_addr zone=nzeza:10m rate=10r/s;

location / {
    limit_req zone=nzeza burst=20 nodelay;
    proxy_pass http://127.0.0.1:3000;
}
```

### 7. Logging and Monitoring

**Enable audit logging for security events:**

```bash
# Set log level
export RUST_LOG=info

# Log to file with rotation
cargo run 2>&1 | tee -a /var/log/nzeza/bot.log
```

**Monitor for:**
- Failed authentication attempts
- Unusual trading patterns
- Unexpected API errors
- Sudden PnL changes

### 8. Update and Patch Management

- Regularly update dependencies: `cargo update`
- Monitor for security advisories: `cargo audit`
- Subscribe to security notifications for exchanges

## 🔒 Production Deployment Checklist

Before deploying to production, ensure:

- [ ] `API_KEYS` environment variable is set to a secure value (32+ chars)
- [ ] TLS/HTTPS is configured (via reverse proxy or VPN)
- [ ] Secrets are stored in a secrets manager (not .env files)
- [ ] Firewall is configured to block direct access to port 3000
- [ ] IP whitelisting is enabled (if applicable)
- [ ] Mnemonic is for a dedicated, funded-only-what-you-risk wallet
- [ ] Logging and monitoring are configured
- [ ] Regular backups of configuration and state are in place
- [ ] dYdX integration is updated to use proper Cosmos signing (currently broken!)

## 🚨 Known Security Limitations

### 1. dYdX v4 Implementation is Incomplete

**Current Status**: The dYdX client uses simplified signing that will NOT work with dYdX v4.

See `src/infrastructure/dydx_client.rs` for details. Do NOT attempt real trades with dYdX until this is fixed.

### 2. No Built-in TLS Support

The application does NOT include built-in TLS. You MUST use a reverse proxy or VPN for production.

### 3. In-Memory State

Currently, all state (positions, PnL, etc.) is stored in memory. On restart:
- Open positions are lost
- PnL history is reset
- You must manually reconcile with exchange state

**Recommendation**: Implement database persistence before production use.

## 📞 Incident Response

If you suspect a security breach:

1. **Immediately stop the bot**: `kill $(pgrep nzeza)`
2. **Rotate API keys**: Generate new keys and update environment
3. **Check wallet transactions**: Review on-chain activity
4. **Review logs**: Check for unauthorized access
5. **Close open positions**: Manually close any remaining positions on exchanges
6. **Investigate**: Determine the attack vector
7. **Patch and redeploy**: Fix the vulnerability before restarting

## 🔗 Additional Resources

- [OWASP API Security Top 10](https://owasp.org/www-project-api-security/)
- [Rust Security Advisory Database](https://rustsec.org/)
- [Let's Encrypt (Free TLS Certificates)](https://letsencrypt.org/)
- [dYdX v4 Documentation](https://docs.dydx.xyz/)

## 📝 License

See LICENSE file for details.
