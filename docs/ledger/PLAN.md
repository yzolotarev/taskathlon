# Accounting module

Goal: a working accounting program inside Taskathlon whose structure matches what an accountant uses every day: catalogs, documents that are posted, double entries, registers, reports and the month-end close. Its relation to commercial accounting software is the same as GridCraft's relation to Excel: behaviour comes from public accounting rules, and everything else is written from scratch.

## Clean room

- Sources: the Russian chart of accounts (Ministry of Finance Order No. 94n), the PBU/FSBU accounting standards, the Tax Code and public descriptions of accounting operations.
- We use no code, configurations, form layouts, icons, texts or object names from any commercial accounting product. Open-source ERPs under the GPL (ERPNext, Tryton, Dolibarr) are a source of ideas only; their code is not used either.
- Subaccount numbers (68.01, 68.02, 90.01 and so on) are common accounting practice, not anyone's code.

## Layers

1. **`crates/ledger` (done):** money in kopecks, dates, the chart of accounts, catalogs, 13 document types, posting and unposting, re-posting in date order (average cost), the month-end close (depreciation, input VAT deduction, closing 20/26/44, financial result 90/91 → 99, profit tax) and reports (trial balance, trial balance by subconto, account card, account analysis, P&L). Includes a demo company.
2. **Program window (done):** sections on the left, document journals, document forms with **Post**, entries view, catalogs and reports with a period picker.
3. **Workshop (done, first set):** 11 timed tasks for February on the demo company: receipts, sales, payments, rent, advances, taxes, payroll, month close and reading numbers off the reports. Each task is checked against the books, so any correct way of doing it passes.

Next: more workshops (error hunting in a trial balance, VAT return figures, inventory count), an English interface.

## Simplifications

Personal income tax at 13% without the progressive scale, insurance contributions at a single 30% rate, profit tax at 25% of the month's profit without PBU 18, no year-end closing of 90/91, no purchase and sales ledgers. VAT is 22% (from 2026).
