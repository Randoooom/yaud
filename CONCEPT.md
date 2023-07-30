# Freelancer Dashboard

# Frontend

## Dashboard

### Statistics

* currently open tasks
* closed tasks by week
* expected profit by week
* profit this month
* currently unclaimed profit / possible profix
* already worked hours today
* worked ours by week

### Tasks list

The task overview should be a well structured and designed table:

| Title | fee | State | started | due              |
|-------|-----|-------|---------|------------------|
| Text  | Num | State | Date    | Date (remaining) |

On click the client should be redirected to the overview for the concerned task.
Here it should also be possible to show already finished tasks by using an switch element.

## Task overview

The client should directly be able to recognize the title of the task. Followed by its detailed description. These
details should be shortened, if they're too long and the client has the option to view / expand the entire description.

On the right or the left should things like tags, current state, received at, due date, fee and other "short" details be
visible. Additionaly a textarea for attaching notes / contacting the customer should be included. Attached note /
messages to or from the customer should be shown.

# Backend definitions

## Objects

### AccessTokenClaims

| Field   | Data type | description    | required | default |
|---------|-----------|----------------|----------|---------|
| session | ID(64)    | the session id | true     | -       |

### Account

| Field      | Data type | description                                | required | default |
|------------|-----------|--------------------------------------------|----------|---------|
| id         | ID        | the internal id                            | false    | ID      |
| first_name | String    | the accounts owner first name              | true     | -       |
| last_name  | String    | the accounts owner last name               | true     | -       |
| mail       | String    | contact e-mail                             | true     | -       |
| password   | String    | double hashed account password             | true     | -       |
| nonce      | String    | the nonce used for the first password hash | true     | -       |
| secret     | String    | the encrypted base32 secret                | true     | -       |

| updated_at | Date | updated at timestamp | false | - |
| created_at | Date | created at timestamp | false | Date |

### ActionLog

| Field      | Data type  | description            | required | default |
|------------|------------|------------------------|----------|---------|
| id         | ID         | the internal id        | false    | ID      |
| type       | ActionType | the type of the action | true     | -       |
| author     | -> Account | the concerned account  | true     | -       |
| target     | -> Object  | the target object      | true     | -       |
| created_at | Date       | created at timestamp   | false    | Date    |

### Comment

| Field      | Data type   | description                | required | default |
|------------|-------------|----------------------------|----------|---------|
| id         | ID          | the internal id            | false    | ID      |
| owner      | -> Account  | the authors account        | true     | -       |
| type       | CommentType | the type of the comment    | true     | -       |
| content    | String      | the content of the comment | true     | -       |
| updated_at | Date        | updated at timestamp       | false    | -       |
| created_at | Date        | created at timestamp       | false    | Date    |

### Permission

| Field | Data type | description                 | required | default |
|-------|-----------|-----------------------------|----------|---------|
| id    | ID        | the internal id             | false    | ID      |
| title | String    | The title of the permission | true     | -       |

### Session

| Field         | Data type  | description                           | required | default |
|---------------|------------|---------------------------------------|----------|---------|
| id            | ID(64)     | the internal id                       | false    | ID(64)  |
| refresh_token | ID(64)     | the refresh token                     | false    | ID(64)  |
| account       | -> Account | the account the session is active for | true     | -       |
| iat           | Date       | initiation timestamp                  | true     | -       |
| exp           | Date       | expiration timestamp                  | true     | -       |

### State

| Field      | Data type | description            | required | default |
|------------|-----------|------------------------|----------|---------|
| id         | ID        | the internal id        | false    | ID      |
| title      | String    | The title of the state | true     | -       |
| updated_at | Date      | last updated timestamp | false    | NONE    |
| created_at | Date      | creation timestamp     | false    | Date    |

### Task

| Field       | Data type    | description                     | required | default |
|-------------|--------------|---------------------------------|----------|---------|
| id          | ID           | the internal id                 | false    | ID      |
| title       | String       | the title of the task           | true     | -       |
| customer    | -> Account   | the account requesting the task | true     | -       |
| description | String       | given description for the task  | true     | -       |
| due         | Option<Date> | due date                        | false    | NONE    |
| state       | State        | the current state               | false    | State   |
| priority    | Priority     | the priority of the task        | false    | 0       |
| created_at  | Date         | creation timestamp              | false    | Date    |
| updated_at  | Date         | updated timestamp               | false    | -       |

### TaskRequest

| Field       | Data type        | description                     | required | default          |
|-------------|------------------|---------------------------------|----------|------------------|
| id          | ID               | the internal id                 | false    | ID               |
| title       | String           | the title of the task           | true     | -                |
| customer    | -> Account       | the account requesting the task | true     | -                |
| description | String           | given description for the task  | true     | -                |
| due         | Option<Date>     | due date                        | false    | NONE             |
| state       | TaskRequestState | the current state               | false    | TaskRequestState |
| created_at  | Date             | creation timestamp              | false    | Date             |
| updated_at  | Date             | updated timestamp               | false    | -                |

## Enums

### AccountType

0. Customer
1. Employee

### ActionType

0. Login
1. Logout
2. TaskRequestIssued
3. TaskRequestRevoked
4. TaskRequestStateChanged
5. TaskStateChanged
6. PermissionGranted
7. PermissionRevoked

### CommentType

0. Note
1. Message

### Priority

0. Low
1. Medium
2. High

### TaskRequestState

0. Received
1. In Evaluation
2. Accepted
3. Rejected

## Graph relations

### Account -> has -> Permission

## Authentication

### Password format

The account password is double hashed using the **argon2** password hashing algorithm. Therefor the **nonce** used for
the first hash is saved as plaintext in the account data. After that the hash data gets hashed one more time using the *
*argon2** algorithm with a random generated **nonce**

### Encryption

The hash data resulting from the first password hash with the saved nonce is used as 32bytes key for the *
*ChaCha20Poly1305** encryption. Encrypted data is always stored in the following format: `iv:base64EncodedData`

### AccountSecret

The secret are 32 randomly generated bytes encoded with base32. The secret as such won't ever be saved in plaintext as
it has to be encrypted in order to serve the integrety for it's TOTP usage.

### TOTP

The TOTP is based on the `AccountSecret` with a **30s** timer interval. With the deactivating of this feature
the `AccountSecret` will automatically be regenerated.

### Session

An valid session allows an connected client to interact with the api features. Sessions are only valid for **15m**
before getting deactivated, but till **5m** after deactivation the client can refresh the session with the
saved `refresh_token`.

## Authorization 
