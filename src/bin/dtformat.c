#include <timezoneapi.h>
#include <math.h>
#include <stdlib.h>

#include "sqlite3ext.h"
SQLITE_EXTENSION_INIT1


static void datetime_format(
  sqlite3_context *pCtx,
  int argc,
  sqlite3_value **argv
){
  sqlite3_int64  *ptr = sqlite3_value_blob(argv[0]);
  sqlite3_int64   v = *ptr;
  SYSTEMTIME      st;

  if (FileTimeToSystemTime((const FILETIME *)ptr, &st)) {
    int     size = 30;
    char  * buf = malloc(size);
    size_t  mult = 10000000;
    double  n;
    size_t  fract = modf((double)v / mult, &n) * mult;

    snprintf (buf, size,"%04d-%02d-%02dT%02d:%02d:%02d.%07dZ", 
              st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond, fract);
    sqlite3_result_text(pCtx, buf, size, free);
  } else {
    sqlite3_result_error(pCtx, "Error", -1);
  }
}

#ifdef _WIN32
__declspec(dllexport)
#endif
int sqlite3_dtformat_init(
  sqlite3 *db, 
  char **pzErrMsg, 
  const sqlite3_api_routines *pApi
){
  int rc = SQLITE_OK;
  SQLITE_EXTENSION_INIT2(pApi);
  rc = sqlite3_create_function(db, "datetime_format", 1, SQLITE_UTF8, 0,
                               datetime_format, 0, 0);
  return rc;
}
