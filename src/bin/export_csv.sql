.open Windows.db
.load dtformat
CREATE TEMP VIEW WorkId as SELECT WorkId FROM SystemIndex_1_PropertyStore;
CREATE TEMP VIEW System_ComputerName as SELECT WorkId, Value as System_ComputerName FROM SystemIndex_1_PropertyStore as a where a.ColumnId=557;
CREATE TEMP VIEW System_ItemPathDisplay as SELECT WorkId, Value as System_ItemPathDisplay FROM SystemIndex_1_PropertyStore as a where a.ColumnId=39;
CREATE TEMP VIEW System_DateModified as SELECT WorkId, datetime_format(Value) as System_DateModified FROM SystemIndex_1_PropertyStore as a where a.ColumnId=441;
CREATE TEMP VIEW System_DateCreated as SELECT WorkId, datetime_format(Value) as System_DateCreated FROM SystemIndex_1_PropertyStore as a where a.ColumnId=445;
CREATE TEMP VIEW System_DateAccessed as SELECT WorkId, datetime_format(Value) as System_DateAccessed FROM SystemIndex_1_PropertyStore as a where a.ColumnId=449;
CREATE TEMP VIEW System_Size as SELECT WorkId, to_int(Value) as System_Size FROM SystemIndex_1_PropertyStore as a where a.ColumnId=436;
CREATE TEMP VIEW System_FileOwner as SELECT WorkId, Value as System_FileOwner FROM SystemIndex_1_PropertyStore as a where a.ColumnId=93;
CREATE TEMP VIEW System_Search_AutoSummary as SELECT WorkId, Value as System_Search_AutoSummary FROM SystemIndex_1_PropertyStore as a where a.ColumnId=303;
CREATE TEMP VIEW System_Search_GatherTime as SELECT WorkId, datetime_format(Value) as System_Search_GatherTime FROM SystemIndex_1_PropertyStore as a where a.ColumnId=26;
CREATE TEMP VIEW System_ItemType as SELECT WorkId, Value as System_ItemType FROM SystemIndex_1_PropertyStore as a where a.ColumnId=567;
CREATE TEMP VIEW System_ItemName as SELECT WorkId, Value as System_ItemName FROM SystemIndex_1_PropertyStore as a where a.ColumnId=318;
CREATE TEMP VIEW System_ItemUrl as SELECT WorkId, Value as System_ItemUrl FROM SystemIndex_1_PropertyStore as a where a.ColumnId=39;
CREATE TEMP VIEW System_ItemDate as SELECT WorkId, datetime_format(Value) as System_ItemDate FROM SystemIndex_1_PropertyStore as a where a.ColumnId=308;
CREATE TEMP VIEW System_ItemFolderNameDisplay as SELECT WorkId, Value as System_ItemFolderNameDisplay FROM SystemIndex_1_PropertyStore as a where a.ColumnId=414;
CREATE TEMP VIEW System_Title as SELECT WorkId, Value as System_Title FROM SystemIndex_1_PropertyStore as a where a.ColumnId=424;
CREATE TEMP VIEW System_Link_DateVisited as SELECT WorkId, datetime_format(Value) as System_Link_DateVisited FROM SystemIndex_1_PropertyStore as a where a.ColumnId=378;
CREATE TEMP VIEW System_ItemNameDisplay as SELECT WorkId, Value as System_ItemNameDisplay FROM SystemIndex_1_PropertyStore as a where a.ColumnId=432;
CREATE TEMP VIEW System_ActivityHistory_StartTime as SELECT WorkId, datetime_format(Value) as System_ActivityHistory_StartTime FROM SystemIndex_1_PropertyStore as a where a.ColumnId=346;
CREATE TEMP VIEW System_ActivityHistory_EndTime as SELECT WorkId, datetime_format(Value) as System_ActivityHistory_EndTime FROM SystemIndex_1_PropertyStore as a where a.ColumnId=341;
CREATE TEMP VIEW System_Activity_AppDisplayName as SELECT WorkId, Value as System_Activity_AppDisplayName FROM SystemIndex_1_PropertyStore as a where a.ColumnId=297;
CREATE TEMP VIEW System_ActivityHistory_AppId as SELECT WorkId, Value as System_ActivityHistory_AppId FROM SystemIndex_1_PropertyStore as a where a.ColumnId=331;
CREATE TEMP VIEW System_Activity_DisplayText as SELECT WorkId, Value as System_Activity_DisplayText FROM SystemIndex_1_PropertyStore as a where a.ColumnId=315;
CREATE TEMP VIEW System_Activity_ContentUri as SELECT WorkId, Value as System_Activity_ContentUri FROM SystemIndex_1_PropertyStore as a where a.ColumnId=311;
CREATE TEMP VIEW VolumeId as SELECT WorkId, get_volume_id(Value) as VolumeId FROM SystemIndex_1_PropertyStore as a where a.ColumnId=311;
CREATE TEMP VIEW ObjectId as SELECT WorkId, get_object_id(Value) as ObjectId FROM SystemIndex_1_PropertyStore as a where a.ColumnId=311;
CREATE TEMP VIEW NamedFields as select * from WorkId as a 
left join System_ComputerName as aa on a.WorkId=aa.WorkId 
left join System_ItemPathDisplay as bb on a.WorkId=bb.WorkId 
left join System_DateModified as b on a.WorkId=b.WorkId 
left join System_DateCreated as c on a.WorkId=c.WorkId 
left join System_DateAccessed as d on a.WorkId=d.WorkId 
left join System_Size as e on a.WorkId=e.WorkId 
left join System_FileOwner as f on a.WorkId=f.WorkId 
left join System_Search_AutoSummary as g on a.WorkId=g.WorkId 
left join System_Search_GatherTime as h on a.WorkId=h.WorkId 
left join System_ItemType as i on a.WorkId=i.WorkId 
left join System_ItemName as j on a.WorkId=j.WorkId 
left join System_ItemUrl as k on a.WorkId=k.WorkId 
left join System_ItemDate as l on a.WorkId=l.WorkId 
left join System_ItemFolderNameDisplay as m on a.WorkId=m.WorkId 
left join System_Title as n on a.WorkId=n.WorkId 
left join System_Link_DateVisited as o on a.WorkId=o.WorkId 
left join System_ItemNameDisplay as p on a.WorkId=p.WorkId 
left join System_ActivityHistory_StartTime as q on a.WorkId=q.WorkId 
left join System_ActivityHistory_EndTime as r on a.WorkId=r.WorkId 
left join System_Activity_AppDisplayName as s on a.WorkId=s.WorkId 
left join System_ActivityHistory_AppId as t on a.WorkId=t.WorkId 
left join System_Activity_DisplayText as u on a.WorkId=u.WorkId 
left join System_Activity_ContentUri as v on a.WorkId=v.WorkId 
left join VolumeId as w on a.WorkId=w.WorkId 
left join ObjectId as x on a.WorkId=x.WorkId 
;
.headers on
.mode csv
.output report.csv
select WorkId, System_ComputerName, System_ItemPathDisplay, System_DateModified, System_DateCreated, System_DateAccessed, System_Size, System_FileOwner, System_Search_AutoSummary, System_Search_GatherTime, System_ItemType, System_ItemName, System_ItemUrl, System_ItemDate, System_ItemFolderNameDisplay, System_Title, System_Link_DateVisited, System_ItemNameDisplay, System_ActivityHistory_StartTime, System_ActivityHistory_EndTime, System_Activity_AppDisplayName, System_ActivityHistory_AppId, System_Activity_DisplayText, System_Activity_ContentUri, VolumeId, ObjectId from NamedFields group by workid;
.exit
