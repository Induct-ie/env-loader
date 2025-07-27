# Environment Loader

A command-line tool that loads environment variables from external sources before executing applications. This enables secure management of secrets and configuration without hardcoding sensitive values.

## Features

- Load secrets from AWS Secrets Manager
- Set literal values with preprocessing
- Pass-through mode for unmodified variables
- Prefix-based variable filtering
- Configurable error handling for missing variables

## Installation

Download the latest binary from the [releases page](https://github.com/Induct-ie/env-loader/releases) or build from source with Rust.

## Usage

```bash
environment-loader [OPTIONS] <COMMAND>...
```

### Command-Line Options

| Option | Short | Description |
|--------|-------|-------------|
| `--pass <VARIABLE>` | `-p` | Variables to pass through unchanged (can be used multiple times) |
| `--ignore-missing` | `-i` | Don't exit when a loadable variable is not found |
| `--env-prefix <PREFIX>` |  | Prefix for environment variables to intercept and process |

### Supported Variable Formats

Environment variables are processed based on their values:

#### Direct Values
```bash
MYVAR="value::my-actual-value"
```
Sets `MYVAR` to `my-actual-value` (strips the `value::` prefix).

#### AWS Secrets Manager
```bash
MYVAR="aws_sm::my-secret-name"
```
Loads the value from AWS Secrets Manager secret named `my-secret-name`.

#### Regular Variables
```bash
MYVAR="regular-value"
```
Passed through unchanged.

## Examples

### Basic Usage

Load a secret from AWS Secrets Manager:
```bash
export DB_PASSWORD="aws_sm::prod/db/password"
environment-loader npm start
```

### Using Literal Values
```bash
export API_URL="value::https://api.example.com"
export DB_HOST="value::localhost"
environment-loader python app.py
```

### Pass-Through Variables

Preserve certain variables unchanged:
```bash
export PATH="/usr/bin:/bin"
export HOME="/home/user"
export DB_PASSWORD="aws_sm::prod/db/password"

environment-loader --pass PATH --pass HOME python app.py
```

### Using Environment Prefix

Process only variables with a specific prefix:
```bash
export REGULAR_VAR="not processed"
export MYAPP_DB_PASSWORD="aws_sm::prod/db/password"
export MYAPP_API_KEY="aws_sm::prod/api/key"
export MYAPP_DEBUG="value::true"

# Only MYAPP_ variables are processed, others pass through unchanged
environment-loader --env-prefix MYAPP_ python app.py
```

In this example:
- `REGULAR_VAR` → passed as `REGULAR_VAR=not processed`
- `MYAPP_DB_PASSWORD` → becomes `DB_PASSWORD=<secret-value>`
- `MYAPP_API_KEY` → becomes `API_KEY=<secret-value>`
- `MYAPP_DEBUG` → becomes `DEBUG=true`

### Ignore Missing Secrets

Continue execution even if secrets can't be loaded:
```bash
export API_KEY="aws_sm::nonexistent-secret"
environment-loader --ignore-missing python app.py
```

### Complex Example

```bash
# Set up environment
export MYAPP_DATABASE_URL="aws_sm::prod/database/url"
export MYAPP_REDIS_URL="aws_sm::prod/redis/url"
export MYAPP_DEBUG="value::false"
export MYAPP_PORT="value::3000"
export PATH="/usr/local/bin:/usr/bin:/bin"
export HOME="/home/user"

# Run application with processed environment
environment-loader \
  --env-prefix MYAPP_ \
  --pass PATH \
  --pass HOME \
  node server.js
```

This will:
1. Load `DATABASE_URL` from AWS Secrets Manager
2. Load `REDIS_URL` from AWS Secrets Manager  
3. Set `DEBUG=false` and `PORT=3000`
4. Pass through `PATH` and `HOME` unchanged
5. Execute `node server.js` with the processed environment

## AWS Configuration

For AWS Secrets Manager integration, ensure your AWS credentials are configured via:
- AWS CLI (`aws configure`)
- Environment variables (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`)
- IAM roles (when running on EC2/ECS/Lambda)
- AWS profiles

Required IAM permissions:
```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": "secretsmanager:GetSecretValue",
      "Resource": "arn:aws:secretsmanager:*:*:secret:*"
    }
  ]
}
```

## Error Handling

By default, the tool exits with code 1 if:
- A required secret cannot be loaded from AWS Secrets Manager
- An unknown load method is specified

Use `--ignore-missing` to continue execution with warnings instead.



