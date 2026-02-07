# Docker Image Registry Guide

## GitHub Container Registry (ghcr.io)

The Manti LLM Gateway Docker images are automatically built and published to GitHub Container Registry (ghcr.io) when changes are pushed to the main branch.

## Image Information

- **Registry:** `ghcr.io`
- **Image Name:** `ghcr.io/[your-github-username]/managua`
- **Available Tags:**
  - `latest` - Latest stable version from main branch
  - `main` - Current main branch build
  - `main-{commit-sha}` - Specific commit build
  - `{date}-{time}` - Timestamped builds (format: YYYYMMDD-HHmmss)

## Pull the Image

### Public Repository

If your repository is public, you can pull the image directly:

```bash
docker pull ghcr.io/[your-github-username]/managua:latest
```

### Private Repository

For private repositories, you need to authenticate first:

```bash
# Login to GitHub Container Registry
echo $GITHUB_TOKEN | docker login ghcr.io -u [your-github-username] --password-stdin

# Pull the image
docker pull ghcr.io/[your-github-username]/managua:latest
```

## Run the Container

### Quick Start

```bash
docker run -d \
  --name manti \
  -p 8080:8080 \
  -e DATABASE_URL="postgres://user:password@host:5432/manti" \
  -e JWT_SECRET="your-secure-jwt-secret" \
  -e MANTI_PROVIDERS_OPENAI_API_KEY="sk-..." \
  ghcr.io/[your-github-username]/managua:latest
```

### Using Docker Compose with Registry Image

Create a `docker-compose.prod.yml` file:

```yaml
version: '3.8'

services:
  postgres:
    image: postgres:16-alpine
    container_name: manti-postgres
    environment:
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: ${DB_PASSWORD}
      POSTGRES_DB: manti
    volumes:
      - postgres_data:/var/lib/postgresql/data
    ports:
      - "5432:5432"
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres"]
      interval: 5s
      timeout: 5s
      retries: 5

  manti:
    image: ghcr.io/[your-github-username]/managua:latest
    container_name: manti-gateway
    depends_on:
      postgres:
        condition: service_healthy
    environment:
      DATABASE_URL: postgres://postgres:${DB_PASSWORD}@postgres:5432/manti
      JWT_SECRET: ${JWT_SECRET}
      MANTI_PROVIDERS_OPENAI_API_KEY: ${OPENAI_API_KEY}
      MANTI_PROVIDERS_ANTHROPIC_API_KEY: ${ANTHROPIC_API_KEY}
      RUST_LOG: ${RUST_LOG:-info}
    ports:
      - "8080:8080"
    restart: unless-stopped

volumes:
  postgres_data:
```

Then run:

```bash
# Create .env file with your secrets
cat > .env <<EOF
DB_PASSWORD=your-secure-password
JWT_SECRET=your-secure-jwt-secret
OPENAI_API_KEY=sk-...
ANTHROPIC_API_KEY=sk-ant-...
EOF

# Run the services
docker-compose -f docker-compose.prod.yml up -d
```

## Environment Variables

Required environment variables:

- `DATABASE_URL` - PostgreSQL connection string
- `JWT_SECRET` - Secret key for JWT token generation

Optional provider configuration:

- `MANTI_PROVIDERS_OPENAI_API_KEY` - OpenAI API key
- `MANTI_PROVIDERS_ANTHROPIC_API_KEY` - Anthropic API key

Server configuration:

- `MANTI_SERVER_HOST` - Server bind address (default: 0.0.0.0)
- `MANTI_SERVER_PORT` - Server port (default: 8080)
- `RUST_LOG` - Log level (default: info)

## Multi-Architecture Support

The CI pipeline builds images for multiple architectures:
- `linux/amd64` - Intel/AMD processors
- `linux/arm64` - ARM processors (Apple Silicon, AWS Graviton)

Docker will automatically pull the correct architecture for your platform.

## CI/CD Pipeline

The GitHub Actions workflow automatically:

1. Triggers on push to main branch
2. Builds the Docker image using multi-stage build
3. Caches layers for faster builds
4. Tags the image with multiple identifiers
5. Pushes to GitHub Container Registry
6. Generates a summary with pull/run commands

## Viewing Available Images

You can view all published images at:
- GitHub Packages: `https://github.com/[your-github-username]/managua/pkgs/container/managua`

## Security Considerations

1. **Never commit secrets** - Use environment variables or secrets management
2. **Use specific tags in production** - Avoid using `latest` in production
3. **Rotate credentials regularly** - Update JWT_SECRET and API keys periodically
4. **Use HTTPS** - Deploy behind a reverse proxy with SSL/TLS
5. **Limit container permissions** - The container runs as non-root user `manti`

## Troubleshooting

### Authentication Issues

If you encounter authentication errors:

```bash
# Generate a personal access token with `read:packages` scope
# Go to: Settings > Developer settings > Personal access tokens

# Login with the token
echo $GITHUB_TOKEN | docker login ghcr.io -u USERNAME --password-stdin
```

### Image Not Found

If the image is not found:
1. Check if the CI pipeline completed successfully
2. Verify the repository name and username
3. Ensure you have pull permissions for private repositories

### Database Connection Issues

If the container can't connect to the database:
1. Verify DATABASE_URL is correct
2. Ensure the database is accessible from the container
3. Check if migrations ran successfully (check container logs)

## Monitoring

View container logs:

```bash
# View logs
docker logs manti

# Follow logs
docker logs -f manti

# View last 100 lines
docker logs --tail 100 manti
```

Check container health:

```bash
# Check status
docker ps

# Inspect container
docker inspect manti

# Check resource usage
docker stats manti
```