# RPC Endpoint Documentation

## Introduction
This document outlines the various endpoints provided by the Will Executor Server, a specialized server designed for managing Bitcoin transactions. Each endpoint can be accessed via HTTP GET or POST 
requests with specific parameters and return values.

## Endpoints

### 1. **Server Information**
- **Endpoint:** `GET /`
- **Description:** Returns general information about the server.
- **Example URL:** `https://we.bitcoin-after.life/`
- **Response:**
  ```plaintext
  Will Executor Server
  ```

### 2. **Server Public Key**
- **Endpoint:** `GET /.pub_key.pem`
- **Description:** Returns the public key of the server.
- **Example URL:** `https://we.bitcoin-after.life/.pub_key.pem`
- **Response:**
  ```plaintext
  -----BEGIN PUBLIC KEY-----
  MCowBQYDK2VwAyEAy10MSrWabdfco1c5Jo1XuohSdXSk1S0YaoEYvqZR5VE=
  -----END PUBLIC KEY-----
  ```

### 3. **Server Version**
- **Endpoint:** `GET /version`
- **Description:** Returns the version of the server.
- **Example URL:** `https://we.bitcoin-after.life/version`
- **Response:**
  ```plaintext
  0.2.2
  ```

### 4. **Network Information**
- **Endpoint:** `GET /<network>/info`
- **Description:** Returns information about a specific network.
- **Example URL:** `https://we.bitcoin-after.life/bitcoin/info`
- **Response:**
  ```json
  {
    "chain": "bitcoin",
    "address": "bc1q5z32sl8at9s3sxt7mfwe6th4jxua98a0mvg8yz",
    "base_fee": 1000,
    "info": "Will Executor Server",
    "version": "0.2.2"
  }
  ```

### 5. **Network Statistics**
- **Endpoint:** `GET /<network>/stats`
- **Description:** Returns statistics for a specific network.
- **Example URL:** `https://we.bitcoin-after.life/bitcoin/stats`
- **Response:**
  ```json
  [
    {
      "report_date": "2025-10-21 03:10:09",
      "chain": "bitcoin",
      "totals": 63,
      "waiting": 8,
      "sent": 30,
      "failed": 25,
      "waiting_profit": 80000,
      "sent_profit": 300000,
      "missed_profit": 250000,
      "unique_inputs": 38
    }
  ]
  ```

### 6. **Search Transaction**
- **Endpoint:** `POST /searchtx`
- **Description:** Searches for a transaction by its ID.
- **Example URL:** `https://we.bitcoin-after.life/searchtx`
- **Request Data:**
  ```plaintext
    241ac86bdbf1408198b8c6df77e88159b43a9bb3464e55197a9fed8fdd628895
  ```
- **Response:**
  ```json
  {
    "status": "1",
    "our_address": "bcrt1q7ajty6q3g055vvy6ryql9y3jz76x5uv806skk6",
    "time": "1733755921197410086",
    "our_fees": "10000",
    "tx": "0200000000010281c28321ff6bcbfcd894bf4536d9a9fb4f4b56470db487da36f8330d495528600100000000fdffffff81c28321ff6bcbfcd894bf4536d9a9fb4f4b56470db487da36f8330d495528600200000000fdffffff031027000000"
  }
  ```

### 7. **Push Transactions**
- **Endpoint:** `POST /push`
- **Description:** Pushes one or more transactions to the network.
- **Example URL:** `https://we.bitcoin-after.life/push`
- **Request Data:**
  ```plaintext
      0200000000010281c28321ff6bcbfcd894bf4536d9a9fb4f4b56470db487da36f8330d495528600100000000fdffffff81c28321ff6bcbfcd894bf4536d9a9fb4f4b56470db487da36f8330d495528600200000000fdffffff031027000000
  ```
- **Response:**
  - If successful, it returns the hash of each transaction:
    ```plaintext
    thx
    ```
  - If a transaction is already present or bad data is received, it returns an error message:
    ```plaintext
    {
      already present // or Bad data received
    }
    ```

## Error Handling
- **400 Bad Request:** Returned when the request contains invalid parameters.
- **500 Internal Server Error:** Returned when the server encounters an unexpected condition.

This documentation should help you effectively interact with the Will Executor Server using its provided endpoints.

