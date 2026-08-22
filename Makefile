srpm:
	curl -sL -o blerust-main.tar.gz https://github.com/SisyphusAeolides/blerust/archive/main/blerust-main.tar.gz
	rpmbuild -bs --define "_sourcedir `pwd`" --define "_srcrpmdir `pwd`" blerust.spec
