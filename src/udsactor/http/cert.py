from .. import types

# Default certificate, will be overwritten by the first call to Broker, it's needed to wake up the server part of the actor
# at the beginning, but will be replaced by the real certificate.
defaultCertificate = types.CertificateInfoType(
    private_key='-----BEGIN ENCRYPTED PRIVATE KEY-----\nMIIFHTBPBgkqhkiG9w0BBQ0wQjApBgkqhkiG9w0BBQwwHAQIfG2+iMYJBswCAggA\nMAwGCCqGSIb3DQIJBQAwFQYJKwYBBAGXVQECBAhCusU5R8ulZQSCBMgheyZ81Qkq\n+TcbPeBlUGCFllSUOo7xQ/OuwYSmzLx8LpN0hQNv4azF6MYH+I8eMSPd3A547yW3\nJE4GjIBfRvcq2X1UZ2FQfECU9UP0ShPuPrVhIh6ZZklmlRjbIF8hGfSzXAuafQb+\n4wXXsofahi/SPgqK1Gw65nRiMcoeRZchJkx8pBgKVWED6Cbh6aAkeqkVKPnsebiV\n6kE+0C7+hgNUbyRd46R+/5NXzPjg4ItfSak+PLzQ1KeRv4Cu6DdzRKJ4V9/MlNdU\nNNEkSVSEaRn4sv+eByU4uxBMaSmD1tLc/A7OmaAeRpIQvls3Zcf2+V0+anAtjbjd\n6eIb2nceey+dKFm4ewlR4mXuzj1QowRTHceOIkvKIrOODxdy9M5hNBZ7VLum29tY\nRhqtmEH2BZZJ8SpM2SsEZzPxqJFiVZbvpeOKjxlMyn1dFWn1rP8uMnfuMKqBaj5D\nd5clOPlwebYw5UpM6Vvawu4nGqxECTSWcfNlDYO5U/0Fsm9+JIrJ7Buukgv2+rhs\nD/6oUK9NB8AW9qnDr7UxbC/ujhkKQG3woaZlPbiMs5WQaS+DrTg4N49wPzS0h+ME\nF8ZzuPnd6+sMGQioCIrQAZ08rk54oCijBhFh8/EQhQKGsMFw2swi9t6+FVU5Bvil\nlhmBd3LA5EuQ5y1X0jRL/+GDiUiZw1gOJP8d/XzhUJL9AmamdqJ6/rAU7lUTNWkM\ndzmFonUO2Mh2zgEEudHsTOH8udZ2l64LIHc6fCkDmM8QzghjrEFyci6R8333DSSM\nwbM0MvyTLM7TTqZUD60EgD+Ihyr/wJcBZY7GVn7hTq7ee14zeI+dZFmTMYOnt0mA\ngof19t0naPPZU+zyl/ambNF5mmSkGOAl4IBHNvPt5ztEVbNpwW3DHbmdYW71Ax+z\nCDlr4iKZahv21o1PCesPV2IlaHZFD6aBRt0DxzMqtq9cpWsI1g7aEaAjRbSvqhMY\npUeqFXz/GfR9rjRkufr48//ll0/Q/Ogx7m1TjQ6mAEQrklI7pa2W0u3H0BpSZSis\nR6ST3ulE+wfsp8cau6q2er+BSsDhBjSn9FeCUjHzY56u9ud/kb6/jLEdgxNpj0na\n3WVqCCCL/dAFSWznBmdracZsRMXapXInHCiiOEkXXbXIXvRKiTPJXdN+w2/U2j2B\nwXZuazVSpmM+xAZTAS9dtBUQJo+5px9b6P09uagvTA32ezbpPXf+hSfmTdUwbmAY\nrmE9SW85tzX+cD17loygBBRrjOr4uQy/s/9FqLx8bM73jly05rdOmX28ECKwEA05\n8aCFkfqrl9J9doVapaUlywpJVPFtE6W6tCF+ULMfb16vEjT1du1+epEnbGGLRQxg\n3aFLyKlvFaNvR38fiQFUGtBgGOaBN3rhGpbMwjch3oReXv9X/4UCL6sVIiOH2H3c\nVSZdC3O5g6CMVe4zckUe1k9mLDb5524IHDFfptZ6Bw+uzrqIy3GHW8dJF2AK471b\nMUnCojTpdbFHaUs2u/rNKVUyY+vLf8hkyP+znBUoPxSJtty53EWNukxjjsxx0lx3\niZGqN72lXlXuSFZAIxi307+xxE21cbzDsMidyJkbKKGm/F4BOKvX9jWmAyYmBG6A\n1L3yNRouFWsYDwYAX2nZ1is=\n-----END ENCRYPTED PRIVATE KEY-----\n',
    server_certificate='-----BEGIN CERTIFICATE-----\nMIIDcTCCAlkCBDfnXU8wDQYJKoZIhvcNAQELBQAwfTELMAkGA1UEBhMCRVMxDzAN\nBgNVBAgMBk1hZHJpZDEPMA0GA1UEBwwGTWFkcmlkMREwDwYDVQQKDAhVRFMgQ2Vy\ndDERMA8GA1UECwwIVURTIENlcnQxEjAQBgNVBAMMCTEyNy4wLjAuMTESMBAGA1Ud\nEQwJMTI3LjAuMC4xMB4XDTIwMDIxNzExNTkzMloXDTMwMDIxNDExNTkzMlowfTEL\nMAkGA1UEBhMCRVMxDzANBgNVBAgMBk1hZHJpZDEPMA0GA1UEBwwGTWFkcmlkMREw\nDwYDVQQKDAhVRFMgQ2VydDERMA8GA1UECwwIVURTIENlcnQxEjAQBgNVBAMMCTEy\nNy4wLjAuMTESMBAGA1UdEQwJMTI3LjAuMC4xMIIBIjANBgkqhkiG9w0BAQEFAAOC\nAQ8AMIIBCgKCAQEA2e1cW7YtRpNLazR3f/LqLv8OB0rKh8cUPH4wuQhbBTkee8Wu\n5eMSadRCIyRbKj4b8dtVfI9QW0SrmhGuMx1KCh3CsYd9XsWiKbGkiRBHIDOn5pkF\n6PUayDJ8KjnGbfnZjp0AmxXP4r1OO8jUPqzKS9Ubf5PgwcwdFiUKVfVPwGwctwt5\nt9YpSRONw0rTsCjVHvO2dd9h6EopskLCWxpN8l9kNLwLM/6t0IqVKmn5/IYPKKN2\nCX8a7IXpxwoiUs4sBZYhUMBWikB1hKQRSYafp1Xvc5PeTFXTFqGANnqz0NoZ8tqL\n8qjQUN/PCdtzhfcP5RgT2g1qyS2RBCMYH7Zs0wIDAQABMA0GCSqGSIb3DQEBCwUA\nA4IBAQCUt+qlLA1N9VXMwDQAYG4Kt6/UlMHCXAajHQQGtjdyGJ4++m7EIjI96hMU\n3Cx2gp2ggR3JGnuSR+DdBvPl5iGku7J8KV0JiJg30gTY8JuUIy/PMLZWloYKrBHV\nlin2GujQ4OsIt3dbr4XtcKW1Wd7L6fBzHlq7Xyxh+gcTzTvTmq67Q9XKlBWsegMf\nv4FKy0lfcSFK3vTzswQtuTontG4TqLiT/4AnMt3D0cTQ6b6KoZwUUX/TDNhau06d\nQ4Ilz8X61ka+4HBkFSR5ahP9noCVhwO329h+6epO141E5Tep3OLc/GCF4oaKOlMR\nfqxf5f2bghU0fxmtEoNJTZkBsN1S\n-----END CERTIFICATE-----\n',
    password='Pw7qbatz5u-y-Z5ora2D2ZuBCm95AHnKRcpze53k8tw',
    ciphers=''
)