FROM httpd:2.4
COPY ./my-httpd.conf /usr/local/apache2/conf/httpd.conf
COPY ./httpd-vhosts.conf /usr/local/apache2/conf/extra/httpd-vhosts.conf
## Serve the test content under /tag at http://localhost/tag
## The build context is tests/resources, so ./tag resolves to tests/resources/tag
COPY ./tag /usr/local/apache2/htdocs/tag