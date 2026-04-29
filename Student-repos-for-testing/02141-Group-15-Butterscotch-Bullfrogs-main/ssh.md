# Setup SSH

This section provides instructions on how to generate an SSH key and add it to your GitLab profile.

### macOS

- Open a terminal
- Run `ssh-keygen`
- Follow the instructions. The defaults should be okay, so just press enter
- Run `cat $HOME/.ssh/id_rsa.pub | pbcopy`
- Goto [https://gitlab.gbar.dtu.dk/-/profile/keys](https://gitlab.gbar.dtu.dk/-/profile/keys)
- Paste into the **Key** text area
- Click **Add key**

### Linux

- Open a terminal
- Run `ssh-keygen`
- Follow the instructions. The defaults should be okay, so just press enter
- Run `cat $HOME/.ssh/id_rsa.pub`
- Copy the printed key
- Goto [https://gitlab.gbar.dtu.dk/-/profile/keys](https://gitlab.gbar.dtu.dk/-/profile/keys)
- Paste into the **Key** text area
- Click **Add key**

### Windows

- Open PowerShell
- Run `ssh-keygen.exe`
- Follow the instructions. The defaults should be okay, so just press enter
- Run `cat $HOME\.ssh\id_rsa.pub | clip`
- Goto [https://gitlab.gbar.dtu.dk/-/profile/keys](https://gitlab.gbar.dtu.dk/-/profile/keys)
- Paste into the **Key** text area
- Click **Add key**
