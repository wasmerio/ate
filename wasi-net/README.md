# WASI Network Interface

This library allows applications compiled to WASI to have access to
HTTP and HTTPS queries that support the interface.

Consuming this library is simple, simply link to it and use the
builder to construct an API request.

On the server side the following must be implemented

1. Create a virtual file under /dev/web
2. Listen for writes to the file that terminate with a \n
3. The first line received is the URL to connect to
4. The second line is the HTTP method to use (e.g. GET,PUT,etc)
5. The third line is an encoded set of headers (base64 encoded JSON representation of a Vec<(String, String)>
6. The last line is the data to set (zero bytes means no data)
7. Then make the HTTP request and allow the file handle to read the data
