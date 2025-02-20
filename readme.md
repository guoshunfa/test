概述：做一个windows端软件，支持多视频合并、音频导入。但我用的是Mac电脑，所以需要考虑程序打包的问题和测试的问题。

技术要求：
1. 使用tauri+vue3+ts
2. 界面描述：
    - 只需要一个界面，界面中展示一个form：
        - 选择文件夹，或者选择视频文件（必须选择一种，给出明确提示）
        - 选择配音音频文件
        - 选择剪辑顺序：按顺序、随机。（先不展示，如果选择的是文件夹，才展示）
        - 选择视频生成时间：40s、50s、60s（先不展示，如果选择的是文件夹，才展示）
        - 开始剪辑按钮
        - （如果选择的是视频文件，则不展示剪辑顺序和视频生成时间）
    - 点击开始剪辑按钮，会出现进度条，进度条加载完成，提示剪辑完成。并弹框提示剪辑后的视频文件路径和音频文件路径
3. 开始剪辑接口描述：
    - 判断选择的是文件夹还是视频文件
      - 如果是文件夹，则拿到所有视频文件
      - 如果是视频文件，则拿到该视频文件
    - 开始合并视频，合并视频的规则：（如果选择的是视频文件，则跳过这个操作）
      - 按照选择的视频生成时间，将视频文件合并、剪辑成对应的时间，如：选择的是40s，则开始按照剪辑顺序，将文件夹下的所有的视频进行批量分配，10个视频为一组，每组视频生成一个视频文件。
      - 并将配音放入到视频中，配音文件时间大于视频文件时间，则裁剪音频到视频文件时间。
    - 返回合并后的视频文件路径和音频文件路径
    - （优化：客户会上传上千个视频，需要分批次进行剪辑，每次剪辑100个视频，剪辑完成后，提示剪辑完成，并弹框提示剪辑后的视频文件路径和音频文件路径）
4. 其他要求：
    - 最终需要把软件打包成exe文件，方便客户使用。

客户操作流程：
1.选择文件夹（文件夹下放入几千个视频），或者选择视频文件
2.选择配音音频文件
3.选择剪辑顺序：按顺序、随机。
4.选择视频生成时间：40s、50s、60s
5.开始剪辑，剪辑完成后会在同目录下生成一个新的文件夹，来存放生成后的视频。
