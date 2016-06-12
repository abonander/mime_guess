#!/bin/sh


#Decrypt RSA key
mkdir -p ~/.ssh
openssl aes-256-cbc -K $encrypted_99f11dfbf8eb_key -iv $encrypted_99f11dfbf8eb_iv -in id_rsa.enc -out ~/.ssh/id_rsa -d
chmod 600 ~/.ssh/id_rsa

git config user.name "mime_guess doc upload"
git config user.email "nobody@example.com"

git checkout --orphan gh-pages

git reset
git clean -d -x -f -e target

cp -R target/doc .
rm -rf target

git add -A

git commit -qm "Documentation for ${TRAVIS_TAG}"
git remote set-url origin git@github.com:cybergeek94/mime_guess.git
git push -f origin gh-pages
