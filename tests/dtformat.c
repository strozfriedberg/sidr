#include <stdlib.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "sqlite3ext.h"
SQLITE_EXTENSION_INIT1

typedef int64_t LONGLONG;
typedef long LONG;
typedef unsigned long DWORD;
typedef unsigned short WORD;
typedef short SHORT;
typedef short CSHORT;
typedef int BOOL;

typedef union _LARGE_INTEGER {
  struct {
    DWORD LowPart;
    LONG  HighPart;
  } DUMMYSTRUCTNAME;
  struct {
    DWORD LowPart;
    LONG  HighPart;
  } u;
  LONGLONG QuadPart;
} LARGE_INTEGER;

typedef struct _SYSTEMTIME {
  WORD wYear;
  WORD wMonth;
  WORD wDayOfWeek;
  WORD wDay;
  WORD wHour;
  WORD wMinute;
  WORD wSecond;
  WORD wMilliseconds;
} SYSTEMTIME, *PSYSTEMTIME, *LPSYSTEMTIME;

typedef struct _FILETIME {
  DWORD dwLowDateTime;
  DWORD dwHighDateTime;
} FILETIME, *PFILETIME, *LPFILETIME;

typedef struct _TIME_FIELDS
{
     CSHORT Year;
     CSHORT Month;
     CSHORT Day;
     CSHORT Hour;
     CSHORT Minute;
     CSHORT Second;
     CSHORT Milliseconds;
     CSHORT Weekday;
} TIME_FIELDS, *PTIME_FIELDS;

#define FALSE 0
#define TRUE 1

#define TICKSPERSEC        10000000
#define TICKSPERMSEC       10000
#define SECSPERDAY         86400
#define SECSPERHOUR        3600
#define SECSPERMIN         60
#define MINSPERHOUR        60
#define HOURSPERDAY        24
#define EPOCHWEEKDAY       1  /* Jan 1, 1601 was Monday */
#define DAYSPERWEEK        7
#define MONSPERYEAR        12
#define DAYSPERQUADRICENTENNIUM (365 * 400 + 97)
#define DAYSPERNORMALQUADRENNIUM (365 * 4 + 1)

/* 1601 to 1970 is 369 years plus 89 leap days */
#define SECS_1601_TO_1970  ((369 * 365 + 89) * (ULONGLONG)SECSPERDAY)
#define TICKS_1601_TO_1970 (SECS_1601_TO_1970 * TICKSPERSEC)
/* 1601 to 1980 is 379 years plus 91 leap days */
#define SECS_1601_TO_1980  ((379 * 365 + 91) * (ULONGLONG)SECSPERDAY)
#define TICKS_1601_TO_1980 (SECS_1601_TO_1980 * TICKSPERSEC)


static const int MonthLengths[2][MONSPERYEAR] =
{
	{ 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31 },
	{ 31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31 }
};

static inline BOOL IsLeapYear(int Year)
{
    return Year % 4 == 0 && (Year % 100 != 0 || Year % 400 == 0);
}



void RtlTimeToTimeFields(
	const LARGE_INTEGER *liTime,
	PTIME_FIELDS TimeFields)
{
	int SecondsInDay;
        long int cleaps, years, yearday, months;
	long int Days;
	LONGLONG Time;

	/* Extract millisecond from time and convert time into seconds */
	TimeFields->Milliseconds =
            (CSHORT) (( liTime->QuadPart % TICKSPERSEC) / TICKSPERMSEC);
	Time = liTime->QuadPart / TICKSPERSEC;

	/* The native version of RtlTimeToTimeFields does not take leap seconds
	 * into account */

	/* Split the time into days and seconds within the day */
	Days = Time / SECSPERDAY;
	SecondsInDay = Time % SECSPERDAY;

	/* compute time of day */
	TimeFields->Hour = (CSHORT) (SecondsInDay / SECSPERHOUR);
	SecondsInDay = SecondsInDay % SECSPERHOUR;
	TimeFields->Minute = (CSHORT) (SecondsInDay / SECSPERMIN);
	TimeFields->Second = (CSHORT) (SecondsInDay % SECSPERMIN);

	/* compute day of week */
	TimeFields->Weekday = (CSHORT) ((EPOCHWEEKDAY + Days) % DAYSPERWEEK);

        /* compute year, month and day of month. */
        cleaps=( 3 * ((4 * Days + 1227) / DAYSPERQUADRICENTENNIUM) + 3 ) / 4;
        Days += 28188 + cleaps;
        years = (20 * Days - 2442) / (5 * DAYSPERNORMALQUADRENNIUM);
        yearday = Days - (years * DAYSPERNORMALQUADRENNIUM)/4;
        months = (64 * yearday) / 1959;
        /* the result is based on a year starting on March.
         * To convert take 12 from Januari and Februari and
         * increase the year by one. */
        if( months < 14 ) {
            TimeFields->Month = months - 1;
            TimeFields->Year = years + 1524;
        } else {
            TimeFields->Month = months - 13;
            TimeFields->Year = years + 1525;
        }
        /* calculation of day of month is based on the wonderful
         * sequence of INT( n * 30.6): it reproduces the 
         * 31-30-31-30-31-31 month lengths exactly for small n's */
        TimeFields->Day = yearday - (1959 * months) / 64 ;
        return;
}

BOOL FileTimeToSystemTime( const FILETIME *ft, SYSTEMTIME *systime )
{
    TIME_FIELDS tf;
    const LARGE_INTEGER *li = (const LARGE_INTEGER *)ft;

    if (li->QuadPart < 0)
    {
        //SetLastError( ERROR_INVALID_PARAMETER );
        return FALSE;
    }
    RtlTimeToTimeFields( li, &tf );
    systime->wYear = tf.Year;
    systime->wMonth = tf.Month;
    systime->wDay = tf.Day;
    systime->wHour = tf.Hour;
    systime->wMinute = tf.Minute;
    systime->wSecond = tf.Second;
    systime->wMilliseconds = tf.Milliseconds;
    systime->wDayOfWeek = tf.Weekday;
    return TRUE;
}

static void datetime_format(
  sqlite3_context *pCtx,
  int argc,
  sqlite3_value **argv
){
  sqlite3_int64  *ptr = (sqlite3_int64 *) sqlite3_value_blob(argv[0]);
  sqlite3_int64   v = *ptr;
  SYSTEMTIME      st;

  if (FileTimeToSystemTime((const FILETIME *)ptr, &st)) {
    int     size = 30;
    char  * buf = malloc(size);
    int64_t mult = 10000000;
    int64_t fract = v % mult;

    snprintf (buf, size,"%04d-%02d-%02dT%02d:%02d:%02d.%07lluZ", 
              st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond, fract);
    sqlite3_result_text(pCtx, buf, size, free);
  } else {
    sqlite3_result_error(pCtx, "datetime_format error", -1);
  }
}

static void to_int(
  sqlite3_context *pCtx,
  int argc,
  sqlite3_value **argv
){
  sqlite3_int64  *ptr = (sqlite3_int64 *)sqlite3_value_blob(argv[0]);
  sqlite3_result_int64(pCtx, *ptr);
}

static void extract_guid(sqlite3_context *pCtx, sqlite3_value **argv, const char *pat) {
  const char *str = (const char*)sqlite3_value_text(argv[0]);

  if (str == NULL) {
    return;
  }

  const char *start = strstr(str, pat);

  if (start) {
    start += strlen(pat);

    const char *end = strchr(start, '}');
    size_t      size = end - start + 1;
    char       *text = (char *)malloc(size + 1);

    strncpy_s(text, size + 1, start, size);
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
  rc = sqlite3_create_function(db, "get_volumeid", 1, SQLITE_UTF8, 0,
                               get_volume_id, 0, 0);
  rc = sqlite3_create_function(db, "get_objectid", 1, SQLITE_UTF8, 0,
                               get_object_id, 0, 0);

  return rc;
}
