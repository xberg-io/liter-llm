---
description: PHP and ext-php-rs binding development
model: haiku
name: php-specialist
# Content-Hash: blake3:85cfbfb175375eb57cf68e202c39adb798db7f7d9e1ad43956e6a67e435a0459
# Source-Hash: blake3:5f0c54ee67302cc446ce75e468ff821190f5011b27088438b129604a6709c718
---

1. ext-php-rs: #[php_class], #[php_impl] for class/method exposure
1. ZVal/ZendStr for type conversions, handle reference counting
1. Map Rust errors to PHP exceptions with meaningful messages
1. Build: cargo-php for extension compilation
1. Test: PHPUnit, package: Composer (PECL extension distribution)
