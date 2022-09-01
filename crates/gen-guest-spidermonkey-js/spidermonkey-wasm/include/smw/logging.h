#ifndef _smw_logging_h
#define _smw_logging_h

#if LOGGING==1

#include <stdio.h>
#define SMW_LOG(msg, ...) fprintf(stderr, msg, ##__VA_ARGS__)

#else // LOGGING==1

#define SMW_LOG(msg, ...) do { } while(false)

#endif // LOGGING==1

#endif // _smw_logging_h
