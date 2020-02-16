
all:
	cargo build

install:
	cp target/debug/hdars /usr/bin/hda-analyzer
	cp app-conf/hda-analyzer.desktop /usr/share/applications/
	cp app-conf/hda-analyzer-pkexec /usr/bin/
	cp app-conf/org.freedesktop.policykit.hda-analyzer.policy /usr/share/polkit-1/actions/

uninstall:
	rm -f /usr/share/applications/hda-analyzer.desktop
	rm -f /usr/bin/hda-analyzer-pkexec
	rm -f /usr/share/polkit-1/actions/org.freedesktop.policykit.hda-analyzer.policy
