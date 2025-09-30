# Rust PostgreSQL CRUD API

A boilerplate for building RESTful APIs with Rust, Axum, and PostgreSQL.

## Features

- RESTful API with CRUD operations
- PostgreSQL database integration with SQLx
- Structured error handling
- Environment-based configuration
- Database migrations
- CORS support
- Docker and Docker Compose support

## Project Structure

```
.
├── migrations/             # Database migrations
├── src/
│   ├── config/             # Application configuration
│   ├── db/                 # Database connection and utilities
│   ├── handlers/           # Request handlers
│   ├── models/             # Data models and schemas
│   ├── routes/             # API routes
│   └── main.rs             # Application entry point
├── .env.example            # Example environment variables
├── Dockerfile              # Docker configuration for the application
├── docker-compose.yml      # Docker Compose configuration
├── Cargo.toml              # Rust dependencies
└── README.md               # Project documentation
```

## Prerequisites

- Rust (latest stable version) - for local development
- PostgreSQL database - for local development
- Docker and Docker Compose - for containerized deployment

## Setup

### Local Development

1. Clone the repository
2. Copy `.env.example` to `.env` and update the database connection string
3. Create a PostgreSQL database
4. Run the application

```bash
# Copy environment file
cp .env.example .env

# Edit .env file with your database credentials
# DATABASE_URL=postgres://username:password@localhost:5432/database_name

# Run the application
cargo run
```

### Docker Deployment

1. Clone the repository
2. Run with Docker Compose

```bash
# Build and start the containers
docker compose up -d

# View logs
docker compose logs -f app

# Stop the containers
docker compose down
```

The application will be available at http://localhost:3000.

#### Docker Notes

- The application uses a multi-stage build process for smaller image size
- The PostgreSQL database is automatically initialized with the required schema
- Database data is persisted in a Docker volume
- The API container will automatically reconnect to the database if it's temporarily unavailable

## API Endpoints

| Method | Endpoint     | Description         |
|--------|--------------|---------------------|
| GET    | /health      | Health check        |
| GET    | /users       | Get all users       |
| POST   | /users       | Create a new user   |
| GET    | /users/:id   | Get user by ID      |
| PUT    | /users/:id   | Update user by ID   |
| DELETE | /users/:id   | Delete user by ID   |

## Example Requests

### Create a User

```bash
curl -X POST http://localhost:3000/users \
  -H "Content-Type: application/json" \
  -d '{"name": "John Doe", "email": "john@example.com"}'
```

### Get All Users

```bash
curl http://localhost:3000/users
```

## Development

To add new features or endpoints:

1. Create models in `src/models/`
2. Add handlers in `src/handlers/`
3. Register routes in `src/routes/mod.rs`
4. Create migrations in `migrations/` folder

## License

MIT