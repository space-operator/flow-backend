FROM docker.io/library/nginx:latest
COPY . /usr/share/nginx/html
RUN rm /usr/share/nginx/html/nginx.conf
RUN rm /usr/share/nginx/html/index.html
COPY nginx.conf /etc/nginx/conf.d/default.conf
