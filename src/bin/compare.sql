.mode csv
.import report.csv rep
.import DESKTOP-O47KVAD_File_Report_20230304_190927.csv wsa
.headers on
select rep.workid, rep.System_ComputerName, wsa.System_ComputerName from rep inner join wsa on rep.workid=wsa.workid where rep.System_ComputerName != wsa.System_ComputerName;
select rep.workid, rep.System_ItemPathDisplay, wsa.System_ItemPathDisplay from rep inner join wsa on rep.workid=wsa.workid where rep.System_ItemPathDisplay != wsa.System_ItemPathDisplay;
select rep.workid, rep.System_DateModified, wsa.System_DateModified from rep inner join wsa on rep.workid=wsa.workid where rep.System_DateModified != wsa.System_DateModified;
select rep.workid, rep.System_DateCreated, wsa.System_DateCreated from rep inner join wsa on rep.workid=wsa.workid where rep.System_DateCreated != wsa.System_DateCreated;
select rep.workid, rep.System_DateAccessed, wsa.System_DateAccessed from rep inner join wsa on rep.workid=wsa.workid where rep.System_DateAccessed != wsa.System_DateAccessed;
select rep.workid, rep.System_Size, wsa.System_Size from rep inner join wsa on rep.workid=wsa.workid where rep.System_Size != wsa.System_Size;
select rep.workid, rep.System_FileOwner, wsa.System_FileOwner from rep inner join wsa on rep.workid=wsa.workid where rep.System_FileOwner != wsa.System_FileOwner;
select rep.workid, rep.System_Search_AutoSummary, wsa.System_Search_AutoSummary from rep inner join wsa on rep.workid=wsa.workid where rep.System_Search_AutoSummary != wsa.System_Search_AutoSummary;
select rep.workid, rep.System_Search_GatherTime, wsa.System_Search_GatherTime from rep inner join wsa on rep.workid=wsa.workid where rep.System_Search_GatherTime != wsa.System_Search_GatherTime;
select rep.workid, rep.System_ItemType, wsa.System_ItemType from rep inner join wsa on rep.workid=wsa.workid where rep.System_ItemType != wsa.System_ItemType;
select rep.workid, rep.System_ItemName, wsa.System_ItemName from rep inner join wsa on rep.workid=wsa.workid where rep.System_ItemName != wsa.System_ItemName;
select rep.workid, rep.System_ItemUrl, wsa.System_ItemUrl from rep inner join wsa on rep.workid=wsa.workid where rep.System_ItemUrl != wsa.System_ItemUrl;
select rep.workid, rep.System_ItemDate, wsa.System_ItemDate from rep inner join wsa on rep.workid=wsa.workid where rep.System_ItemDate != wsa.System_ItemDate;
select rep.workid, rep.System_ItemFolderNameDisplay, wsa.System_ItemFolderNameDisplay from rep inner join wsa on rep.workid=wsa.workid where rep.System_ItemFolderNameDisplay != wsa.System_ItemFolderNameDisplay;
select rep.workid, rep.System_Title, wsa.System_Title from rep inner join wsa on rep.workid=wsa.workid where rep.System_Title != wsa.System_Title;
select rep.workid, rep.System_Link_DateVisited, wsa.System_Link_DateVisited from rep inner join wsa on rep.workid=wsa.workid where rep.System_Link_DateVisited != wsa.System_Link_DateVisited;
select rep.workid, rep.System_ItemNameDisplay, wsa.System_ItemNameDisplay from rep inner join wsa on rep.workid=wsa.workid where rep.System_ItemNameDisplay != wsa.System_ItemNameDisplay;
select rep.workid, rep.System_ActivityHistory_StartTime, wsa.System_ActivityHistory_StartTime from rep inner join wsa on rep.workid=wsa.workid where rep.System_ActivityHistory_StartTime != wsa.System_ActivityHistory_StartTime;
select rep.workid, rep.System_ActivityHistory_EndTime, wsa.System_ActivityHistory_EndTime from rep inner join wsa on rep.workid=wsa.workid where rep.System_ActivityHistory_EndTime != wsa.System_ActivityHistory_EndTime;
select rep.workid, rep.System_Activity_AppDisplayName, wsa.System_Activity_AppDisplayName from rep inner join wsa on rep.workid=wsa.workid where rep.System_Activity_AppDisplayName != wsa.System_Activity_AppDisplayName;
select rep.workid, rep.System_ActivityHistory_AppId, wsa.System_ActivityHistory_AppId from rep inner join wsa on rep.workid=wsa.workid where rep.System_ActivityHistory_AppId != wsa.System_ActivityHistory_AppId;
select rep.workid, rep.System_Activity_DisplayText, wsa.System_Activity_DisplayText from rep inner join wsa on rep.workid=wsa.workid where rep.System_Activity_DisplayText != wsa.System_Activity_DisplayText;
select rep.workid, rep.System_Activity_ContentUri, wsa.System_Activity_ContentUri from rep inner join wsa on rep.workid=wsa.workid where rep.System_Activity_ContentUri != wsa.System_Activity_ContentUri;
select rep.workid, rep.VolumeId, wsa.VolumeId from rep inner join wsa on rep.workid=wsa.workid where rep.VolumeId != wsa.VolumeId;
select rep.workid, rep.ObjectId, wsa.ObjectId from rep inner join wsa on rep.workid=wsa.workid where rep.ObjectId != wsa.ObjectId;
