# AUR package

`gestione-attivita-bin` installs the release package from GitHub system-wide: `/usr/bin/gestione-attivita`, the icon, the two menu entries of `install.sh` and the license. The AUR repository only holds the three files of [gestione-attivita-bin](gestione-attivita-bin): `PKGBUILD`, `.SRCINFO` and `gestione-attivita-bin.install`.

CI (`aur` job in `.github/workflows/ci.yml`) builds this PKGBUILD against the release it points to, checks it with namcap and fails if `.SRCINFO` is out of date.

## First submission

1. Create an account on [aur.archlinux.org](https://aur.archlinux.org) and add your SSH public key to it (My Account → SSH Public Key).
2. Clone the new, empty package and copy the files into it:

   ```bash
   git clone ssh://aur@aur.archlinux.org/gestione-attivita-bin.git aur-gestione-attivita-bin
   cp packaging/aur/gestione-attivita-bin/{PKGBUILD,.SRCINFO,gestione-attivita-bin.install} aur-gestione-attivita-bin/
   cd aur-gestione-attivita-bin
   git add PKGBUILD .SRCINFO gestione-attivita-bin.install
   git commit -m "Initial import: gestione-attivita-bin 2.2.0-1"
   git push origin master
   ```

## After each release

Once the release workflow has attached `gestione-attivita-X.Y.Z-linux-x86_64.tar.gz` to the GitHub release (its job summary shows the SHA-256):

```bash
cd packaging/aur/gestione-attivita-bin
sed -i 's/^pkgver=.*/pkgver=X.Y.Z/; s/^pkgrel=.*/pkgrel=1/' PKGBUILD
updpkgsums                          # from pacman-contrib
makepkg --printsrcinfo > .SRCINFO
makepkg -f && namcap PKGBUILD ./*.pkg.tar.zst
```

Commit the three files here, then copy them to the AUR clone, commit and push. For a change to the packaging alone, raise `pkgrel` instead of `pkgver`.
