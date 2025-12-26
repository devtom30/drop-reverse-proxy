docker build -t dropofculture-apache .
docker run -dit --name dropofculture-apache \
-p 127.0.0.1:8084:80 \
-v ./tag:/usr/local/apache2/htdocs/tag \
dropofculture-apache