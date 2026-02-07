#!/bin/bash

# Production Deployment Script for Manti LLM Gateway

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
COMPOSE_FILE="docker-compose.prod.yml"
ENV_FILE=".env"
ENV_EXAMPLE=".env.prod.example"

# Functions
print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

print_info() {
    echo -e "ℹ $1"
}

check_requirements() {
    print_info "Checking requirements..."

    # Check Docker
    if ! command -v docker &> /dev/null; then
        print_error "Docker is not installed"
        exit 1
    fi
    print_success "Docker is installed"

    # Check Docker Compose
    if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
        print_error "Docker Compose is not installed"
        exit 1
    fi
    print_success "Docker Compose is installed"

    # Check environment file
    if [ ! -f "$ENV_FILE" ]; then
        print_error "Environment file $ENV_FILE not found"
        print_info "Creating from template..."
        cp "$ENV_EXAMPLE" "$ENV_FILE"
        print_warning "Please edit $ENV_FILE with your configuration before running again"
        exit 1
    fi
    print_success "Environment file exists"

    # Validate required environment variables
    required_vars=("GITHUB_USERNAME" "DB_PASSWORD" "JWT_SECRET")
    for var in "${required_vars[@]}"; do
        if ! grep -q "^${var}=" "$ENV_FILE" || grep -q "^${var}=change-this" "$ENV_FILE"; then
            print_error "Required variable $var is not configured in $ENV_FILE"
            exit 1
        fi
    done
    print_success "Required environment variables are configured"
}

pull_latest_image() {
    print_info "Pulling latest image from GitHub Container Registry..."

    # Load environment variables
    source "$ENV_FILE"

    # Check if we need to login to ghcr.io
    if [ -n "$GITHUB_TOKEN" ]; then
        print_info "Logging in to GitHub Container Registry..."
        echo "$GITHUB_TOKEN" | docker login ghcr.io -u "$GITHUB_USERNAME" --password-stdin
    fi

    # Pull the image
    IMAGE="ghcr.io/${GITHUB_USERNAME}/managua:${IMAGE_TAG:-latest}"
    print_info "Pulling image: $IMAGE"
    docker pull "$IMAGE"
    print_success "Image pulled successfully"
}

deploy() {
    local profile="$1"

    print_info "Starting deployment..."

    # Build command based on profile
    cmd="docker-compose -f $COMPOSE_FILE"

    case "$profile" in
        "basic")
            print_info "Deploying basic configuration..."
            ;;
        "ssl")
            print_info "Deploying with SSL support..."
            cmd="$cmd --profile with-ssl"
            ;;
        "cache")
            print_info "Deploying with Redis cache..."
            cmd="$cmd --profile with-cache"
            ;;
        "monitoring")
            print_info "Deploying with monitoring stack..."
            cmd="$cmd --profile monitoring"
            ;;
        "full")
            print_info "Deploying full stack..."
            cmd="$cmd --profile with-ssl --profile with-cache --profile monitoring"
            ;;
        *)
            print_error "Invalid profile: $profile"
            print_info "Available profiles: basic, ssl, cache, monitoring, full"
            exit 1
            ;;
    esac

    # Deploy
    $cmd up -d
    print_success "Deployment completed"

    # Show status
    sleep 3
    $cmd ps
}

stop_services() {
    print_info "Stopping services..."
    docker-compose -f $COMPOSE_FILE down
    print_success "Services stopped"
}

restart_services() {
    print_info "Restarting services..."
    docker-compose -f $COMPOSE_FILE restart
    print_success "Services restarted"
}

show_logs() {
    local service="$1"
    local lines="${2:-100}"

    if [ -z "$service" ]; then
        docker-compose -f $COMPOSE_FILE logs --tail="$lines" -f
    else
        docker-compose -f $COMPOSE_FILE logs --tail="$lines" -f "$service"
    fi
}

backup_database() {
    print_info "Creating database backup..."

    # Create backup directory
    BACKUP_DIR="backups/$(date +%Y%m%d)"
    mkdir -p "$BACKUP_DIR"

    # Generate backup filename
    BACKUP_FILE="$BACKUP_DIR/manti_$(date +%Y%m%d_%H%M%S).sql"

    # Perform backup
    docker exec manti-postgres pg_dump -U postgres manti > "$BACKUP_FILE"

    # Compress backup
    gzip "$BACKUP_FILE"

    print_success "Backup created: ${BACKUP_FILE}.gz"
}

restore_database() {
    local backup_file="$1"

    if [ -z "$backup_file" ]; then
        print_error "Please provide a backup file"
        exit 1
    fi

    if [ ! -f "$backup_file" ]; then
        print_error "Backup file not found: $backup_file"
        exit 1
    fi

    print_warning "This will restore the database from: $backup_file"
    print_warning "All current data will be lost!"
    read -p "Are you sure? (yes/no): " confirm

    if [ "$confirm" != "yes" ]; then
        print_info "Restore cancelled"
        exit 0
    fi

    print_info "Restoring database..."

    # Decompress if needed
    if [[ "$backup_file" == *.gz ]]; then
        gunzip -c "$backup_file" | docker exec -i manti-postgres psql -U postgres manti
    else
        docker exec -i manti-postgres psql -U postgres manti < "$backup_file"
    fi

    print_success "Database restored"
}

health_check() {
    print_info "Checking service health..."

    # Check PostgreSQL
    if docker exec manti-postgres pg_isready -U postgres &> /dev/null; then
        print_success "PostgreSQL is healthy"
    else
        print_error "PostgreSQL is not responding"
    fi

    # Check Manti API
    if curl -s -o /dev/null -w "%{http_code}" http://localhost:8080/health | grep -q "200"; then
        print_success "Manti API is healthy"
    else
        print_error "Manti API is not responding"
    fi

    # Show container status
    docker-compose -f $COMPOSE_FILE ps
}

show_help() {
    echo "Manti LLM Gateway - Production Deployment Script"
    echo ""
    echo "Usage: $0 [command] [options]"
    echo ""
    echo "Commands:"
    echo "  deploy [profile]   Deploy services (profiles: basic, ssl, cache, monitoring, full)"
    echo "  stop              Stop all services"
    echo "  restart           Restart all services"
    echo "  pull              Pull latest image from registry"
    echo "  logs [service]    Show logs (optionally for specific service)"
    echo "  backup            Create database backup"
    echo "  restore [file]    Restore database from backup"
    echo "  health            Check service health"
    echo "  help              Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 deploy basic           # Deploy basic configuration"
    echo "  $0 deploy full            # Deploy with all features"
    echo "  $0 logs manti             # Show Manti service logs"
    echo "  $0 backup                 # Create database backup"
    echo "  $0 restore backups/file.sql.gz  # Restore from backup"
}

# Main script
case "$1" in
    deploy)
        check_requirements
        pull_latest_image
        deploy "${2:-basic}"
        health_check
        ;;
    stop)
        stop_services
        ;;
    restart)
        restart_services
        ;;
    pull)
        check_requirements
        pull_latest_image
        ;;
    logs)
        show_logs "$2" "$3"
        ;;
    backup)
        backup_database
        ;;
    restore)
        restore_database "$2"
        ;;
    health)
        health_check
        ;;
    help|--help|-h)
        show_help
        ;;
    *)
        print_error "Invalid command: $1"
        show_help
        exit 1
        ;;
esac