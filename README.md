# Rust PostgreSQL API

Backend service built with Rust, Axum, and PostgreSQL. It provides user CRUD, a flexible website crawler, and social media proxy integrations (TikHub and RapidAPI).

## Features

- User CRUD API with PostgreSQL via SQLx
- Website crawler:
  - Multiple domains (comma-separated `url` input)
  - Keyword matching with contextual snippets
  - Pagination following, depth/page limits, and time limits
  - Optional date range filtering (`date_from`, `date_to`)
  - Aggregated full cleaned page content and metadata
- Social media proxy integrations:
  - TikHub Twitter (web): `fetch_search_timeline` with `keyword` and `search_type`
  - TikHub TikTok (web): `fetch_search_video` with `keyword`, `count`, `offset`
  - TikHub Generic proxy for any TikHub service
  - RapidAPI Instagram proxy
  - RapidAPI Twitter v24 proxy
  - RapidAPI Generic proxy for other hosts
- Structured error handling
- Environment-based configuration
- CORS support (GET/POST/PUT/DELETE; `Authorization`, `Accept`, `Content-Type`)
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
# SERVER_PORT=3000
# TIKHUB_TOKEN=your_tikhub_token_here
# RAPIDAPI_KEY=your_rapidapi_key_here

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

| Method | Endpoint                          | Description                                   |
|--------|-----------------------------------|-----------------------------------------------|
| GET    | `/health`                         | Health check                                  |
| GET    | `/users`                          | Get all users                                 |
| POST   | `/users`                          | Create a new user                             |
| GET    | `/users/{id}`                     | Get user by ID                                |
| PUT    | `/users/{id}`                     | Update user by ID                             |
| DELETE | `/users/{id}`                     | Delete user by ID                             |
| POST   | `/crawl`                          | Crawl websites and extract keyword contexts   |
| POST   | `/social/tikhub/generic`          | Proxy for TikHub generic services             |
| POST   | `/social/tikhub/twitter`          | Proxy for TikHub Twitter (web)                |
| POST   | `/social/tikhub/tiktok`           | Proxy for TikHub TikTok (web)                 |
| POST   | `/social/rapidapi/instagram`      | Proxy for RapidAPI Instagram                  |
| POST   | `/social/rapidapi/twitter-v24`    | Proxy for RapidAPI Twitter v24                |
| POST   | `/social/rapidapi/generic`        | Proxy for arbitrary RapidAPI hosts            |

## Environment Variables

- `DATABASE_URL` (required): PostgreSQL connection string
- `SERVER_PORT` (optional, default `3000`): Server port
- `TIKHUB_TOKEN` (required for TikHub proxies): TikHub API token
- `RAPIDAPI_KEY` (required for RapidAPI proxies): RapidAPI key

## Crawler Endpoint

- Endpoint: `POST /crawl`
- Request body (JSON):

```json
{
  "url": "https://example.com, https://another.com",
  "keywords": ["rust", "axum"],
  "max_depth": 2,
  "max_time_seconds": 30,
  "follow_pagination": true,
  "max_pages": 10,
  "date_from": "2024-01-01",
  "date_to": "2024-12-31"
}
```

- Notes:
  - `url` can be a single URL or multiple comma-separated URLs.
  - `follow_pagination` enables automatic next-page discovery.
  - `max_pages` and `max_time_seconds` act as hard limits; results will include `has_more_pages` when limits are hit.

- Example:

```bash
curl -X POST http://localhost:3000/crawl \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://example.com",
    "keywords": ["rust", "axum"],
    "follow_pagination": true,
    "max_pages": 5
  }'
```

## Social Proxies

All social proxies accept a JSON body with:

```json
{
  "path": "endpoint/path",
  "params": { "key": "value" },
  "method": "GET" | "POST"
}
```

### TikHub Generic

- Endpoint: `POST /social/tikhub/generic`
- Body:

```json
{
  "service": "twitter/web", // or "tiktok/web", etc.
  "path": "endpoint/path",
  "params": { "key": "value" },
  "method": "GET"
}
```

- Example:

```bash
curl -X POST http://localhost:3000/social/tikhub/generic \
  -H "Content-Type: application/json" \
  -d '{
    "service": "twitter/web",
    "path": "fetch_search_timeline",
    "method": "GET",
    "params": { "keyword": "mbg", "search_type": "Top" }
  }'
```

### TikHub Twitter (web)

- Endpoint: `POST /social/tikhub/twitter`
- Default base: `https://api.tikhub.io/api/v1/twitter/web/`
- Query mapping:
  - Accepts `keyword` directly; also maps `q` → `keyword` if provided.
  - Defaults `search_type` to `Top` when not provided.

- Example:

```bash
curl -X POST http://localhost:3000/social/tikhub/twitter \
  -H "Content-Type: application/json" \
  -d '{
    "path": "fetch_search_timeline",
    "method": "GET",
    "params": { "keyword": "mbg", "search_type": "Top" }
  }'
```

### TikHub TikTok (web)

- Endpoint: `POST /social/tikhub/tiktok`
- Default base: `https://api.tikhub.io/api/v1/tiktok/web/`
- Query mapping:
  - Accepts `keyword` directly; also maps `q` → `keyword` if provided.
  - Defaults: `count = 20`, `offset = 0` when not provided.

- Example:

```bash
curl -X POST http://localhost:3000/social/tikhub/tiktok \
  -H "Content-Type: application/json" \
  -d '{
    "path": "fetch_search_video",
    "method": "GET",
    "params": { "keyword": "mbg", "count": 20, "offset": 0 }
  }'
```

### RapidAPI Instagram

- Endpoint: `POST /social/rapidapi/instagram`
- Base host: `instagram-scraper-api2.p.rapidapi.com`
- Example:

```bash
curl -X POST http://localhost:3000/social/rapidapi/instagram \
  -H "Content-Type: application/json" \
  -d '{
    "path": "some/endpoint",
    "method": "GET",
    "params": { "key": "value" }
  }'
```

### RapidAPI Twitter v24

- Endpoint: `POST /social/rapidapi/twitter-v24`
- Base host: `twitter-v24.p.rapidapi.com`
- Example:

```bash
curl -X POST http://localhost:3000/social/rapidapi/twitter-v24 \
  -H "Content-Type: application/json" \
  -d '{
    "path": "some/endpoint",
    "method": "GET",
    "params": { "key": "value" }
  }'
```

### RapidAPI Generic

- Endpoint: `POST /social/rapidapi/generic`
- Body:

```json
{
  "host": "your-rapidapi-host",
  "path": "endpoint/path",
  "params": { "key": "value" },
  "method": "GET"
}
```

- Example:

```bash
curl -X POST http://localhost:3000/social/rapidapi/generic \
  -H "Content-Type: application/json" \
  -d '{
    "host": "twitter-v24.p.rapidapi.com",
    "path": "some/endpoint",
    "method": "GET",
    "params": { "key": "value" }
  }'
```

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

## Kafka Producer and Cassandra Consumer Workflow

This project includes a Kafka-based pipeline to persist crawl outputs into Cassandra.

### Overview

- Backend (`/crawl`) emits `CrawlResult` JSON to Kafka when `KAFKA_BROKERS` and `KAFKA_TOPIC_CRAWL` are set.
- Consumer (`consumer/`) subscribes to the crawl topic and writes records into Cassandra.

### Environment Variables

- `KAFKA_BROKERS`: Kafka bootstrap servers, e.g. `broker:9092`.
- `KAFKA_TOPIC_CRAWL`: Topic name to publish/consume, e.g. `crawl_results`.
- `CASSANDRA_CONTACT_POINTS`: Cassandra contact points, e.g. `cassandra`.
- `CASSANDRA_KEYSPACE`: Keyspace for data, e.g. `scraper`.

### Docker Compose Services

- `broker`: Kafka (KRaft single-node) exposed on `localhost:9092`.
- `cassandra`: Cassandra exposed on `localhost:9042`.
- `consumer`: Rust service that consumes `KAFKA_TOPIC_CRAWL` and inserts rows.

### Backend Producer Behavior

- On successful crawl, the backend publishes the full result JSON to Kafka.
- Failures to deliver to Kafka are logged but do not fail the HTTP request.

### Cassandra Schema

- On startup, the consumer ensures the keyspace and a table exist:
  - `scraper.crawl_results (id uuid PRIMARY KEY, payload text, created_at timestamp)`
  - Adjust schema for production needs (partitioning, indexes).

### Quick Test

1. `docker compose up -d` from repo root.
2. Send a crawl request (see examples above).
3. Tail consumer logs: `docker compose logs -f consumer`.
4. Verify in Cassandra:

```bash
docker compose exec cassandra cqlsh -e "SELECT count(*) FROM scraper.crawl_results;"
```