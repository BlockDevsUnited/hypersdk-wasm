CC=clang
CFLAGS=-I.
DEPS=callbacks.h types.h
OBJ=callbacks.o

%.o: %.c $(DEPS)
	$(CC) -c -o $@ $< $(CFLAGS)

libcallbacks.a: $(OBJ)
	ar rcs $@ $^

.PHONY: clean

clean:
	rm -f *.o *.a
