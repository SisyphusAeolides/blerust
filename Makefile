srpm:
	dnf install -y rpmdevtools || true
	spectool -g blerust.spec
	rpmbuild -bs --define "_sourcedir `pwd`" --define "_srcrpmdir `pwd`" blerust.spec
