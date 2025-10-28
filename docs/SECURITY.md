# Security Guidelines

This document provides comprehensive security guidelines for deploying and operating the NZEZA trading system in production.

## Critical Security Requirements

### ⚠️ NEVER Deploy Without These

The NZEZA trading bot **MUST NOT** be exposed to the public internet without:

1. **TLS/HTTPS encryption** via reverse proxy
2. **Strong API authentication** (32+ character keys)
3. **Rate limiting** (default: 100 req/min)
4. **Network isolation** (firewall rules, VPN, or private network)

**Failure to implement these safeguards exposes:**
- API keys in plaintext over the network
- Trading orders and strategies to eavesdroppers  
- Portfolio balances and positions
- Exchange account credentials

## TLS/HTTPS Configuration

The application listens on `127.0.0.1:3000` (HTTP only). **You MUST use a reverse proxy for TLS termination.**

### Option 1: Nginx with Let's Encrypt (Recommended)

Complete Nginx configuration with automatic HTTPS:

\`\`\`nginx
# /etc/nginx/sites-available/nzeza-trading
server {
    listen 443 ssl http2;
    server_name trading.yourdomain.com;

    # Let's Encrypt certificates
    ssl_certificate /etc/letsencrypt/live/trading.yourdomain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/trading.yourdomain.com/privkey.pem;

    # Modern TLS configuration
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers 'ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256';
    ssl_prefer_server_ciphers off;

    # Security headers
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;

    # Proxy to trading application
    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
        proxy_read_timeout 90s;
    }

    # WebSocket support
    location /ws/ {
        proxy_pass http://127.0.0.1:3000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_read_timeout 3600s;
    }
}

# Redirect HTTP to HTTPS
server {
    listen 80;
    server_name trading.yourdomain.com;
    return 301 https://\$server_name\$request_uri;
}
\`\`\`

**Setup commands:**
\`\`\`bash
# Install and configure
sudo apt-get install certbot python3-certbot-nginx
sudo certbot --nginx -d trading.yourdomain.com
sudo nginx -t && sudo systemctl reload nginx
\`\`\`

### Option 2: Caddy (Automatic HTTPS)

Caddy handles TLS certificates automatically:

\`\`\`caddyfile
trading.yourdomain.com {
    reverse_proxy localhost:3000
}
\`\`\`

### Option 3: Private Network + VPN

- Deploy on private VPC/subnet
- Access via VPN (WireGuard, OpenVPN, Tailscale)
- **Never expose port 3000 to public internet**

## API Key Management

### Generating Secure Keys

\`\`\`bash
# Generate 32+ character cryptographically secure key
openssl rand -base64 32

# Multiple keys (comma-separated for key rotation)
export API_KEYS=\$(openssl rand -base64 32),\$(openssl rand -base64 32)
\`\`\`

### Key Rotation (Every 90 Days)

\`\`\`bash
# 1. Generate new key
NEW_KEY=\$(openssl rand -base64 32)

# 2. Add to existing (transition period)
export API_KEYS="\$NEW_KEY,\$OLD_KEYS"
systemctl restart nzeza-trading

# 3. After clients migrate, remove old keys
export API_KEYS="\$NEW_KEY"
systemctl restart nzeza-trading
\`\`\`

## Secrets Management

### ❌ DO NOT Use .env Files in Production

Use a proper secrets manager:

**Option 1: HashiCorp Vault**
\`\`\`bash
vault kv put secret/nzeza-trading \\
    api_keys="\$(openssl rand -base64 32)" \\
    dydx_mnemonic="word1 word2..."
\`\`\`

**Option 2: AWS Secrets Manager**
\`\`\`bash
aws secretsmanager create-secret \\
    --name nzeza-trading/api-keys \\
    --secret-string "\$(openssl rand -base64 32)"
\`\`\`

**Option 3: Kubernetes Secrets**
\`\`\`yaml
apiVersion: v1
kind: Secret
metadata:
  name: nzeza-trading-secrets
stringData:
  api-keys: "your-secure-key-here"
\`\`\`

## Network Security

### Firewall Configuration

\`\`\`bash
# Block direct access to application port
sudo ufw default deny incoming
sudo ufw allow 22/tcp    # SSH
sudo ufw allow 443/tcp   # HTTPS via reverse proxy
# DO NOT: sudo ufw allow 3000/tcp  # ❌ Never expose directly
sudo ufw enable
\`\`\`

## Security Fixes Included

✅ **Implemented in this version:**
- **API key logging removed** - Prevents credential leakage in logs
- **Slippage protection** - Converts market orders to limit orders with price bounds
- **Race condition fixes** - Atomic position limit checks prevent concurrent limit bypass
- **Circuit breakers** - Background tasks auto-recover from failures (no silent degradation)
- **Rate limiting** - 100 requests/minute default
- **Strong authentication** - Enforces 32+ character API keys

## Production Deployment Checklist

Before going live:

- [ ] TLS/HTTPS enabled via reverse proxy
- [ ] API keys are 32+ characters, stored in vault
- [ ] Firewall blocks direct access to port 3000
- [ ] Rate limiting configured and tested
- [ ] Security headers configured (HSTS, CSP)
- [ ] Logging enabled for auth failures
- [ ] Monitoring configured for security events
- [ ] Regular key rotation schedule (90 days)
- [ ] Non-root user running application

## Monitoring Security Events

Watch for these patterns in logs:

\`\`\`bash
# Failed authentication attempts
journalctl -u nzeza-trading | grep "Invalid API key"

# Rate limit violations
journalctl -u nzeza-trading | grep "Rate limit exceeded"

# Circuit breaker trips (critical failures)
journalctl -u nzeza-trading | grep "exceeded maximum consecutive failures"
\`\`\`

## Reporting Security Issues

Found a vulnerability? **DO NOT** open a public issue.

Contact: [Configure your security contact]

Include:
- Description and impact
- Steps to reproduce
- Suggested fix (optional)

Response time: 48 hours

## Additional Resources

- [OWASP API Security Top 10](https://owasp.org/www-project-api-security/)
- [Let's Encrypt Documentation](https://letsencrypt.org/docs/)
- [Nginx Security Best Practices](https://www.nginx.com/blog/mitigating-ddos-attacks-with-nginx-and-nginx-plus/)
