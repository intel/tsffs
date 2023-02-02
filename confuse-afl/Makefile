CC=afl-clang
CXX=afl-clang++
CFLAGS = -g --std=c++11
SOURCES = confuse-afl-wrapper.cpp fake_harness.cpp
EXEC = instrumentedTest

all: $(SOURCES)
	${CXX} ${CFLAGS} ${SOURCES} -o ${EXEC}

clean: instrumentedTest
	rm instrumentedTest