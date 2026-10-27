pwdx for Windows (command line)
===============================

Turns one memorable password into a different, long password for every site. Offline: nothing is stored or sent.

1. INSTALL
   Unzip this folder anywhere, for example C:\Tools\pwdx. There is nothing to install.
   (Optional) add that folder to your PATH so you can type "pwdx" from any folder:
   Settings > System > About > Advanced system settings > Environment Variables > Path > Edit > New.

2. CHECK IT WORKS
   Open PowerShell or Command Prompt, go to the folder, and run:
       .\pwdx.exe selftest
   Expected: selftest ok (spec v1)

3. CREATE YOUR KEY (once)
       .\pwdx.exe key new
   You choose a key-file passphrase (at least 12 characters). The encrypted key file is saved at:
       %APPDATA%\pwdx\key.pwdxkey
   BACK THIS FILE UP. Without it, all your derived passwords are lost.
   To use your existing SSH key instead:
       .\pwdx.exe key import-ssh $env:USERPROFILE\.ssh\id_ed25519

4. GET A PASSWORD FOR A SITE
       .\pwdx.exe get github.com --user you@example.com
   It asks for the key-file passphrase and your memorable password (typing is hidden), then prints the password.
   Copy it straight to the clipboard:
       .\pwdx.exe get github.com --user you@example.com | clip
   Options: --profile alnum|full|passphrase|pin   --length N   --counter N   --key PATH
   Use the SAME site, username, counter, profile and length every time for the same account.

5. SAME KEY ON OTHER DEVICES
   Copy key.pwdxkey to the other device (the Linux and Windows versions use the same file format).
   Check that "pwdx key id" shows the same 8-character Key ID everywhere.

WINDOWS WARNINGS
   - This program is not code-signed, so Windows SmartScreen may say "Windows protected your PC".
     Choose "More info" > "Run anyway" only if you downloaded it from the official place and the checksum matches:
         Get-FileHash .\pwdx-0.1.0-windows-x86_64.zip -Algorithm SHA256
   - Some antivirus programs may flag unsigned command-line tools. Compare the checksum with the one published by the author.

Free software under the MIT License (see LICENSE). It comes with NO WARRANTY: you are responsible for keeping your key file,
its backup, your key-file passphrase and your memorable password safe. See also THIRD_PARTY_LICENSES.txt.
