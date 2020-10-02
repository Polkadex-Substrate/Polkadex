FROM node:latest
COPY ./engine-tests/ /engine-tests
RUN cd /engine-tests/ && npm install
CMD node /engine-tests/basic-tests.js