#include <timezoneapi.h>
#include <math.h>
#include <stdlib.h>

#include "sqlite3ext.h"
SQLITE_EXTENSION_INIT1


static void to_int(
  sqlite3_context *pCtx,
  int argc,
  sqlite3_value **argv
){
  sqlite3_int64  *ptr = sqlite3_value_blob(argv[0]);
  sqlite3_result_int64(pCtx, *ptr);
}

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
    size_t  fract = v % mult;

    snprintf (buf, size,"%04d-%02d-%02dT%02d:%02d:%02d.%07dZ", 
              st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond, fract);
    sqlite3_result_text(pCtx, buf, size, free);
  } else {
    sqlite3_result_error(pCtx, "datetime_format error", -1);
  }
}

static void extract_guid(sqlite3_context *pCtx, sqlite3_value **argv, const char *pat) {
  const char *str = (const char*)sqlite3_value_text(argv[0]);
  if (str == NULL)
    return;

  const char *start = strstr(str, pat) + strlen(pat);

  if (start) {
    const char *end = strchr(start, '}');
    size_t      size = end - start + 1;
    char       *text = (char *)malloc(size + 1);
    
    strncpy(text, start, size);
    sqlite3_result_text(pCtx, text, size, free);
  } 
}

static void get_volume_id(
  sqlite3_context *pCtx,
  int argc,
  sqlite3_value **argv
){
  extract_guid(pCtx, argv, "VolumeId=");
}

static void get_object_id(
  sqlite3_context *pCtx,
  int argc,
  sqlite3_value **argv
){
  extract_guid(pCtx, argv, "ObjectId=");
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
  rc = sqlite3_create_function(db, "to_int", 1, SQLITE_UTF8, 0,
                               to_int, 0, 0);
  rc = sqlite3_create_function(db, "datetime_format", 1, SQLITE_UTF8, 0,
                               datetime_format, 0, 0);
  rc = sqlite3_create_function(db, "get_volume_id", 1, SQLITE_UTF8, 0,
                               get_volume_id, 0, 0);
  rc = sqlite3_create_function(db, "get_object_id", 1, SQLITE_UTF8, 0,
                               get_object_id, 0, 0);

  return rc;
}
