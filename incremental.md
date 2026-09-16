New requirements on UI workflow

- fix: the tags pane should contain only DISTINCT existing tags, case: i edited a bookmark which previously had the tag "programming", once saved none of the bookmark had such tag but the tag was still showing in the left pane
- we need to constrain and polish the rendering of each item so they have a consistent hight and the use the right estate available. My recommendation is that we divide the space of each item vertically in 3 section: 1) left most, include title and URL. the URL can be cropped so that it only shows the domain name the title could also be cropped to two lines so exceedingly long names aren't floodding the UI 2) description, this is also wrapped textbook which would have up to 5 lines because text is smaller, 3) tags and update button. 
- Overall we can stretch the UI to accomodate more information if needed, let's start with a first rendering and we'll finetune
