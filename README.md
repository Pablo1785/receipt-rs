# Receipt-rs

Backend app for quickly storing receipt information, with a focus on robustness, flexibility and ease of integration.

Intended for use with mobile automation apps like HTTP Shortcuts and Macrodroid.

## Features

### Upload receipt photo

- identifies and extracts receipt entries, date, store name
- stores it in the DB

### Get all data as .csv

## Implementation

- uses Azure OCR to process photos:
    - manually implemented the Azure Document Intelligence SDK in Rust:
        - added some tricky Azure Responses as test cases for the parsing logic
        - caches raw OCR responses - if the parsing logic is wrong, no data is lost; DB data will be repopulated periodically from cache, if a new app version fixed the issue, the data will become available automatically some time after release

- simple design for easy integrations:
    - bearer token auth allows to easily connect to the app using mobile automation apps, such as HTTP Shortcuts or Macrodroid
    - conforms to correct HTTP status codes - not a "500 for everything" approach

- non-blocking REST API:
    - uploading a photo schedules a background job, returns success status if job started correctly
