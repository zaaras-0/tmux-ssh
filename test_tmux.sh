#!/bin/bash
tmux -L test_zbw new-session -d -s test_session -n test_window
# Create a dummy executable for "ssh"
export PATH="$PWD/dummy_bin:$PATH"
mkdir -p dummy_bin
echo '#!/bin/bash' > dummy_bin/ssh
echo 'echo "Dummy SSH Started"; sleep 5' >> dummy_bin/ssh
chmod +x dummy_bin/ssh

# We need a dummy config so zbw can start
# Actually, since it requires the vault, I will just compile a small Rust program to mimic zbw _connect setting the options and then exec-ing ssh.
