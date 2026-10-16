# Direktiven: Anzeige und Entdeckung

Stand: 16. Oktober 2026. Referenz: [Screaming Frog – Konfiguration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/).

Die dokumentierten Optionen Respect Noindex und Respect Canonical blenden betroffene URLs in den Berichten aus. Respect Next/Prev betrifft Folgeseiten mit einem prev-Verweis. Abruf und Verfolgung ausgehender Links bleiben dabei erhalten. Die getrennten Crawl-Einstellungen für Canonical- und Pagination-Ziele regeln deren Entdeckung.

Unser früheres Kriterium für `robots.traversal` vermischte diese Aufgaben. Das korrigierte Kriterium bleibt offen; es wurde keine neue Produktfunktion implementiert.

Für die Umsetzung sind unabhängige gespeicherte Anzeigeschalter, unveränderte Rohbeobachtungen, nachvollziehbare sichtbare/erfasste Zählungen und Tests nach einem Neustart vorgesehen. Fixtures müssen sicherstellen, dass ein Anzeigefilter weder Outlinks unterdrückt noch neue Abrufe verursacht. Bestehende Nofollow-Regeln und die Entdeckung von Beziehungs-URLs sind separat zu prüfen.
