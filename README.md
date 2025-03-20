# SWUpdate HTTPd

Minimal HTTP server that provides update images for [SWUpdate](https://sbabic.github.io/swupdate/swupdate.html)
clients.

The server listens on a given IP address and port and provides a root endpoint `/` that can be used by SWUpdate
clients to check for available updates and serves the update images then on the endpoint `/images`.

The server is implemented using the [Actix web framework](https://actix.rs/).

## Update Request

The server expects the following parameters in the query string of the request to `/`:
- `image`: Identifier of the image (to distinguish different images / applications).
- `device`: Identifier of the hardware / device type.
- `current_version`: The currently installed version of the image. An update image is only provided by the server
   if the version of the image on the server is different to the given version.

### Example

A request could look like this:

```shell
curl -v "http://localhost:8080/?image=test-app&device=raspberrypi0-2w-64&current_version=0.1.0"
```

### Response

The server might respond with any of the following HTTP status codes:
- *302 (found)*: An update is available. The response contains a `Location` header that points to the update image.
- *400 (bad request)*: The request is missing one or more of the parameters.
- *404 (not found)*: No update is available.
- *500 (internal server error)*: The server could not successfully process the request. E.g., there was more than
  one matching update image placed in the images directory.

### Processing in the Server

The server reads all files from the images directory (given as a command line parameter at server starting) and
filters them based on the request parameters. It expects that either one or zero update images will match the
filter criteria.

## Image Serving

The update images are served on the `/images` endpoint. The server reads all files from a given directory and
serves them under this path. The images are served as static files and can be downloaded by the clients.
Clients would usually use the `Location` header from the response to the update request to download the update
image.

To provide an update image, the file just needs to be placed in the images directory. There need to be a few rules
followed, however, to make the server work correctly:
- The filename of the update image needs to contain the following fields separated by the given separator (which by
  default is `_`):
  - The image identifier
  - The device type
  - The version number
 - There must always be only one update image available for a given image identifier and device type. If there are
   more than one, the server will respond with a 500 (internal server error) status code.

## Starting and Configuring the Server

The minimal command line for starting the server is as follows:

```shell
swupdate-httpd --images_directory /path/to/update-images
```
In addition, the following optional parameters can be provided:
- `listen_ip`: The interface to listen on.
- `listen_port`: The port to listen on.
- `filename_fields_separator`: The separator used in the filename to separate the fields.
- `filename_field_image_identifier`: The index of the field in the filename that contains the image identifier.
- `filename_field_device_type`: The index of the field in the filename that contains the device type.
- `filename_field_version`: The index of the field in the filename that contains the version number.

