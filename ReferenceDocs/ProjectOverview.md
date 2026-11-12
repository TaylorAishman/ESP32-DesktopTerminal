Programming languages:
Rust -> Application Layer
C -> Hardware Abstraction Layer

UX:
User will be presented with four screens that they can cycle to with physical buttons present on the desktop terminals. Four buttons will be present which will map to a home page, weather page, news page, and finance page. The home page will consist of highlights from the other three dedicated pages. The pages should have graphics and icons to allow for an enjoyable viewing experience. The information presented on the pages should be laid out in a comprehensible way that would be able to be shipped as a finalized product. Topics should be configurable by user in a template file. There should be another template file for the user to input their api keys into along with wifi credentials that can be ignored by github/AI in order to not expose them. 

Button mapping:
1 -> Home 
2 -> Weather
3 -> News
4 -> Finance

Weather:
Daily forecast and hourly forecast

News:
US and global news/politics, finance and economics, technology

Finance:
DJIA, VOO, VXUS, VTI

Technical specs:
API calls should be handled by a manager/scheduler. There should be a config file that allows me to specify my API call limits and the URL's should be abstracted out into this config file. The API manager/scheduler should make API requests, process into data models. Rendering onto the eink display should simply request information from the data model. Using C as a hardware abstraction layer should minimize the instances of unsafe Rust. Errors should be handled gracefully, error handling should endeavor to never brick/lock the system and the error message should be informative enough to allow the user to debug with minimal effort. 