from pypozyx import *

# Import Pose, Point and Quarternion as msg type, to differentiate from Pozyx classes of same name
from geometry_msgs.msg import Pose as MsgPose
from geometry_msgs.msg import Point as MsgPoint
from geometry_msgs.msg import Quaternion as MsgQuaternion

import yaml


class PozyxLocalizer:
    """
    Pozyx localizer class

    This class connects to a pozyx tag through a USB serial connection and returns the current position from the pozyx
    environment.

    Attributes
    ----------
    deviceID: int
        The ID of the pozyx tag in hexadecimal or decimal form.
    pose: MsgPose
        The position and orientation of the pozyx device
    remoteID: int, optional
        The pozyx remote id, used to connect to multiple pozyx tags.
    remote: boolean, optional
        Set to True if using multiple tags. Defaults to False.
    algorithm: int, optional
        The type of pozyx positionging algorithm. POZYX_POS_ALG_UWB_ONLY is enabled by default.
    anchors: DeviceCoordinates[]
        List of Coordinates objects. Coordinates is explaining the coordinates of the anchors in the Pozyx system.
    dimension: int, optional
        The amount of dimentions to use. Either 2D, 3D, 5D.
    height: int, optional
        The height of the Pozyx system in mm. This is only used when the dimentions is in 3D. Defaults to 1000mm.

    Methods
    -------
    parseYamlConfig(path)
        Parsing the configuration file to the self.anchors as a list of anchors.
    createSerialConnectionToTag(tagName=None)
        Creating a serial connection either automatically or manually with providing the tagName. Returns a PozyxSerial object.
    setAnchorsManually(saveToFlash=False)
        Using anchors that is provided in self.anchors.
    loop()
        Getting the position of the connected tag and returning it.
    posAndOrientatonToString():
        Returning a string with the x,y,z coordinates of the position and the orientation.


    """
    def __init__(self, anchors, port=None):
        """
        Attributes
        ----------
        deviceID: int
            The ID of the pozyx tag in hexadecimal or decimal form.
        pose: MsgPose
            The position and orientation of the pozyx device
        remoteID: int, optional
            The pozyx remote id, used to connect to multiple pozyx tags.
        remote: boolean, optional
            Set to True if using multiple tags. Defaults to False.
        algorithm: int, optional
            The type of pozyx positionging algorithm. POZYX_POS_ALG_UWB_ONLY is enabled by default.
        anchors: Coordinates[]
            List of Coordinates objects. Coordinates is explaining the coordinates of the anchors in the Pozyx system.
        dimension: int, optional
            The amount of dimentions to use. Either 2D, 3D, 5D.
        height: int, optional
            The height of the Pozyx system in mm. This is only used when the dimentions is in 3D. Defaults to 1000mm.
        """

        self.pose = MsgPose()   # Position and orientation of the Pozyx tag in a ROS Pose type

        self.pozyx = None           # Pozyx class
        self.anchors = anchors

        if type(self.anchors) == str:
            self.parseYamlConfig(self.anchors)

        if port is None:
            self.pozyx = self.createSerialConnectionToTag()
        else:
            self.pozyx = self.createSerialConnectionToTag(tagName=port)

        self.setAnchorsManually()

    def parseYamlConfig(self, path):
        """
        Parsing the configuration file to the self.anchors as a list of anchors.

        Parameters
        ----------
        path: str
            The path of the config file.
        """
        anchors = []

        with open(path, "r") as file:
            configYaml = yaml.safe_load(file)
            for anchor in configYaml["anchors"]:
                coordinates = Coordinates(anchor["coordinates"]["x"], anchor["coordinates"]["y"], anchor["coordinates"]["z"])
                dc = DeviceCoordinates(anchor["id"], anchor["flag"], coordinates)
                anchors.append(dc)

        self.anchors = anchors

    def createSerialConnectionToTag(self, tagName=None):
        """
        Creating a serial connection either automatically or manually with providing the tagName. Returns a PozyxSerial object.

        Paramters
        ---------
        tagName:
            The name of the serial port. Use this variable for connecting to a USB port manually. If not used, this method will detect the USB port automatically.
        """
        if tagName is None:
            serialPort = get_first_pozyx_serial_port()
            print('Auto assigning serial port: ' + str(serialPort))
        else:
            serialPort = tagName

        connection = PozyxSerial(serialPort)

        if connection is None:
            print('No Pozyx connected. Check if one is connected')

        return connection

    def setAnchorsManually(self, saveToFlash=False):
        """
        Using anchors that is provided in self.anchors.

        Paramters
        ---------
        saveToFlash: boolean, optional
            If set to True. The tag will save the anchors posisions to it's flash memory.
        """
        status = self.pozyx.clearDevices()

        for anchor in self.anchors:
            status &= self.pozyx.addDevice(anchor)

        if len(self.anchors) > 4:
            status &= self.pozyx.setSelectionOfAnchors(POZYX_ANCHOR_SEL_AUTO, len(self.anchors))

        if saveToFlash:
            self.pozyx.saveAnchorIds()
            self.pozyx.saveRegisters([PozyxRegisters.POSITIONING_NUMBER_OF_ANCHORS])

        return status

    def loop(self):
        """
        Getting the position of the connected tag and returning it.
        """
        # Define variable to store Position and orientation
        position = Coordinates()
        orientation = Quaternion()

        # Set position and orientation
        status = self.pozyx.doPositioning(position, PozyxConstants.DIMENSION_2D)
        self.pozyx.getQuaternion(orientation)

        # Set ROS pose to values form Pozyx
        self.pose = MsgPose(
            position=MsgPoint(x=position.x, y=position.y, z=position.z),
            orientation=MsgQuaternion(x=orientation.x, y=orientation.y, z=orientation.z, w=orientation.w)
        )

        if status != POZYX_SUCCESS:
            statusString = "failure" if status == POZYX_FAILURE else "timeout"
            print('Error: Do positioning failed due to ' + statusString)


if __name__ == '__main__':
    localizer = PozyxLocalizer("PozyxConfig.yaml")

    while True:
        localizer.loop()
