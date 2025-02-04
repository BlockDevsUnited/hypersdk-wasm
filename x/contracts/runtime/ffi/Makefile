.PHONY: all clean

UNAME := $(shell uname)

ifeq ($(UNAME), Darwin)
	LIBEXT = dylib
else
	LIBEXT = so
endif

all: libhypersdk.$(LIBEXT)

libhypersdk.$(LIBEXT): bridge.go
	go build -buildmode=c-shared -o $@ $<

clean:
	rm -f libhypersdk.* *.h
